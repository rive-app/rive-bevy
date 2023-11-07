use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    math::Vec3A,
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::Face,
        view::RenderLayers,
    },
};

use crate::{
    components::{MeshEntity, RiveLinearAnimation, RiveStateMachine, SpriteEntity, Viewport},
    plugin::get_scene_or,
};

#[derive(Debug)]
enum CameraType {
    Camera2d,
    Camera3d,
}

fn get_filter_map_for_sprite(
    image_dimensions: Vec2,
    transform: Transform,
) -> impl Fn(Vec2) -> Option<Vec2> + Copy {
    let scaled_image_dimension = image_dimensions * transform.scale.truncate();
    let bounding_box =
        Rect::from_center_size(transform.translation.truncate(), scaled_image_dimension);

    move |world_pos| {
        bounding_box.contains(world_pos).then(|| {
            let mut pos: Vec2 = world_pos - bounding_box.min;
            // Flip y since the Y axis points down in Rive.
            pos.y = bounding_box.height() - pos.y;
            pos /= transform.scale.truncate();

            pos
        })
    }
}

#[derive(Debug)]
struct PointerEventPasser<'e> {
    cursor_moved_events: Vec<&'e CursorMoved>,
    mouse_button_input_events: Vec<&'e MouseButtonInput>,
}

impl<'e> PointerEventPasser<'e> {
    pub fn new(
        cursor_moved_events: &'e mut EventReader<CursorMoved>,
        mouse_button_input_events: &'e mut EventReader<MouseButtonInput>,
    ) -> Self {
        Self {
            cursor_moved_events: cursor_moved_events.read().collect(),
            mouse_button_input_events: mouse_button_input_events.read().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cursor_moved_events.is_empty() && self.mouse_button_input_events.is_empty()
    }

    pub fn pass<F: Fn(Vec2) -> Option<Vec2> + Copy>(
        &mut self,
        filter_map: F,
        windows: &Query<&Window>,
        scene: &mut dyn rive_rs::Scene,
        viewport: &Viewport,
    ) {
        self.cursor_moved_events.retain(|cursor_moved| {
            if let Some(pos) = filter_map(cursor_moved.position) {
                scene.pointer_move(pos.x, pos.y, viewport);

                false
            } else {
                true
            }
        });

        self.mouse_button_input_events.retain(|mouse_button_input| {
            if mouse_button_input.button != MouseButton::Left {
                return true;
            }

            if let Some(pos) = windows
                .get(mouse_button_input.window)
                .ok()
                .and_then(|w| w.cursor_position())
                .and_then(filter_map)
            {
                match mouse_button_input.state {
                    ButtonState::Pressed => scene.pointer_down(pos.x, pos.y, viewport),
                    ButtonState::Released => scene.pointer_down(pos.x, pos.y, viewport),
                }

                false
            } else {
                true
            }
        });
    }
}

#[derive(Debug)]
struct Triangle {
    vertices: [Vec3; 3],
    uvs: [Vec2; 3],
}

impl Triangle {
    pub fn intersect_to_mesh_uv(&self, ray: Ray, cull_mode: Option<Face>) -> Option<Vec2> {
        let edge0: Vec3A = (self.vertices[1] - self.vertices[0]).into();
        let edge1: Vec3A = (self.vertices[2] - self.vertices[0]).into();
        let ray_direction: Vec3A = ray.direction.into();
        let p_vec = ray_direction.cross(edge1);
        let det: f32 = edge0.dot(p_vec);

        let culled_det = match cull_mode {
            Some(Face::Front) => -det,
            Some(Face::Back) => det,
            None => det.abs(),
        };

        if culled_det < f32::EPSILON {
            return None;
        }

        let det_recip = det.recip();

        let t_vec: Vec3A = (ray.origin - self.vertices[0]).into();
        let u = t_vec.dot(p_vec) * det_recip;

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q_vec = t_vec.cross(edge0);
        let v = ray_direction.dot(q_vec) * det_recip;

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let uv = self.uvs[1].mul_add(
            Vec2::splat(u),
            self.uvs[2].mul_add(Vec2::splat(v), (1.0 - u - v) * self.uvs[0]),
        );

        Some(uv)
    }
}

#[derive(Debug)]
struct Triangles<'m> {
    mesh: &'m Mesh,
    transform: &'m Transform,
    i: usize,
}

impl<'m> Triangles<'m> {
    pub fn new(mesh: &'m Mesh, transform: &'m Transform) -> Self {
        Self {
            mesh,
            transform,
            i: 0,
        }
    }
}

impl Iterator for Triangles<'_> {
    type Item = Triangle;

    fn next(&mut self) -> Option<Self::Item> {
        fn get_nth_triangle<U: Copy>(indices: &[U], n: usize) -> Option<[U; 3]> {
            indices.chunks_exact(3).nth(n)?.try_into().ok()
        }

        let indices: [u32; 3] = self.mesh.indices().and_then(|indices| match indices {
            Indices::U16(indices) => get_nth_triangle(indices, self.i).map(|a| a.map(Into::into)),
            Indices::U32(indices) => get_nth_triangle(indices, self.i),
        })?;

        let vertices = self.mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
        let VertexAttributeValues::Float32x2(uvs) = self.mesh.attribute(Mesh::ATTRIBUTE_UV_0)?
        else {
            return None;
        };

        let vertices = [
            vertices.get(indices[0] as usize)?,
            vertices.get(indices[1] as usize)?,
            vertices.get(indices[2] as usize)?,
        ]
        .map(|&a| self.transform.transform_point(Vec3::from_array(a)));
        let uvs = [
            uvs.get(indices[0] as usize)?,
            uvs.get(indices[1] as usize)?,
            uvs.get(indices[2] as usize)?,
        ]
        .map(|&a| Vec2::from_array(a));

        self.i += 1;

        Some(Triangle { vertices, uvs })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pass(
    cameras: Query<(
        &Camera,
        &GlobalTransform,
        Option<&Camera2d>,
        Option<&RenderLayers>,
    )>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    windows: Query<&Window>,
    mut scenes: Query<(
        Option<&mut RiveLinearAnimation>,
        Option<&mut RiveStateMachine>,
        &Handle<Image>,
        &SpriteEntity,
        &MeshEntity,
        &Viewport,
    )>,
    image_assets: Res<Assets<Image>>,
    sprites: Query<(&Transform, Option<&RenderLayers>), With<Sprite>>,
    meshes: Query<(
        &Transform,
        &Handle<Mesh>,
        &Handle<StandardMaterial>,
        Option<&RenderLayers>,
    )>,
    mesh_assets: Res<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
) {
    let mut cameras: Vec<_> = cameras
        .iter()
        .map(|(camera, camera_transform, camera_2d, render_layers)| {
            (
                camera,
                camera_transform,
                if camera_2d.is_some() {
                    CameraType::Camera2d
                } else {
                    CameraType::Camera3d
                },
                render_layers.copied().unwrap_or(RenderLayers::all()),
            )
        })
        .collect();
    cameras.sort_by_key(|t| t.0.order);

    let mut passer =
        PointerEventPasser::new(&mut cursor_moved_events, &mut mouse_button_input_events);

    for (camera, camera_transform, camera_type, camera_render_layers) in cameras {
        for (linear_animation, state_machine, image_handle, sprite_entity, mesh_entity, viewport) in
            &mut scenes
        {
            if passer.is_empty() {
                break;
            }

            let mut scene = get_scene_or!(continue, linear_animation, state_machine);
            let image_dimensions = image_assets.get(image_handle).unwrap().size().as_vec2();

            match camera_type {
                CameraType::Camera2d => {
                    let Some((transform, render_layers)) = sprite_entity
                        .entity
                        .and_then(|entity| sprites.get(entity).ok())
                    else {
                        continue;
                    };

                    if !camera_render_layers
                        .intersects(&render_layers.copied().unwrap_or(RenderLayers::all()))
                    {
                        continue;
                    }

                    passer.pass(
                        |pos| {
                            camera
                                .viewport_to_world(camera_transform, pos)
                                .map(|ray| ray.origin.truncate())
                                .and_then(get_filter_map_for_sprite(image_dimensions, *transform))
                        },
                        &windows,
                        &mut *scene,
                        viewport,
                    );
                }
                CameraType::Camera3d => {
                    let Some((transform, mesh_handle, material_handle, render_layers)) =
                        mesh_entity
                            .entity
                            .and_then(|entity| meshes.get(entity).ok())
                    else {
                        continue;
                    };

                    let Some(mesh) = mesh_assets.get(mesh_handle) else {
                        continue;
                    };

                    let Some(material) = material_assets.get(material_handle) else {
                        continue;
                    };

                    if !camera_render_layers
                        .intersects(&render_layers.copied().unwrap_or(RenderLayers::all()))
                    {
                        continue;
                    }

                    for triangle in Triangles::new(mesh, transform) {
                        if passer.is_empty() {
                            break;
                        }

                        passer.pass(
                            |pos| {
                                camera
                                    .viewport_to_world(camera_transform, pos)
                                    .and_then(|ray| {
                                        triangle.intersect_to_mesh_uv(ray, material.cull_mode)
                                    })
                                    .map(|pos| pos * image_dimensions)
                            },
                            &windows,
                            &mut *scene,
                            viewport,
                        );
                    }
                }
            }
        }
    }
}
