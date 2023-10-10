use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::{
    core_pipeline::{core_2d, core_3d},
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraphApp},
        render_resource::{
            CommandEncoderDescriptor, Extent3d, ImageCopyTexture, Origin3d, Texture, TextureAspect,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderQueue},
        Render, RenderApp, RenderSet,
    },
};
use rive_rs::Instantiate;
use vello::{RenderParams, Renderer, RendererOptions};

use crate::{
    assets::{self, Artboard, Riv, RivLoader},
    components::{
        LinearAnimation, RiveLinearAnimation, RiveStateMachine, SpriteEntity, StateMachine,
        VelloFragment, VelloScene, Viewport,
    },
    events::{GenericEvent, Input, InputValue},
};

macro_rules! get_scene_or_continue {
    ( $linear_animation:expr, $state_machine:expr ) => {{
        let linear_animation = $linear_animation
            .map(|la| la.map_unchanged(|la| (&mut **la) as &mut dyn rive_rs::Scene));
        let state_machine =
            $state_machine.map(|sm| sm.map_unchanged(|sm| (&mut **sm) as &mut dyn rive_rs::Scene));

        match (linear_animation, state_machine) {
            (Some(linear_animation), None) => linear_animation,
            (None, Some(state_machine)) => state_machine,
            _ => continue,
        }
    }};
}

fn insert_deafult_viewports(
    mut commands: Commands,
    query: Query<
        (Entity, &Handle<Image>),
        (
            Or<(Added<LinearAnimation>, Added<StateMachine>)>,
            Without<Viewport>,
        ),
    >,
    image_assets: Res<Assets<Image>>,
) {
    for (entity, image_handle) in &query {
        let mut viewport = Viewport::default();

        if let Some(image) = image_assets.get(image_handle) {
            let size = image.size();
            viewport.resize(size.x as u32, size.y as u32);
        }

        commands.entity(entity).insert(viewport);
    }
}

fn resize_viewports(
    mut query: Query<(&mut Viewport, &Handle<Image>)>,
    image_assets: Res<Assets<Image>>,
) {
    for (mut viewport, image_handle) in &mut query {
        if let Some(image) = image_assets.get(image_handle) {
            let size = image.size();
            viewport.resize(size.x as u32, size.y as u32);
        }
    }
}

#[derive(Debug, Default, Deref, DerefMut, Resource)]
struct ArtboardEntities(HashMap<AssetId<assets::Artboard>, Entity>);

fn instantiate_linear_animations(
    mut commands: Commands,
    query: Query<(Entity, &LinearAnimation), Without<RiveLinearAnimation>>,
    artboard_assets: Res<Assets<assets::Artboard>>,
    mut artboard_entities: ResMut<ArtboardEntities>,
) {
    for (entity, linear_animation) in &query {
        if let Some(artboard) = artboard_assets.get(&linear_animation.artboard) {
            let handle = linear_animation.artboard.clone();
            let linear_animation =
                rive_rs::LinearAnimation::instantiate(&artboard, linear_animation.index).unwrap();

            commands
                .entity(entity)
                .insert(RiveLinearAnimation(linear_animation));

            artboard_entities.insert(handle.id(), entity);
        }
    }
}

fn instantiate_state_machines(
    mut commands: Commands,
    query: Query<(Entity, &StateMachine), Without<RiveStateMachine>>,
    artboard_assets: Res<Assets<assets::Artboard>>,
    mut artboard_entities: ResMut<ArtboardEntities>,
) {
    for (entity, state_machine) in &query {
        if let Some(artboard) = artboard_assets.get(&state_machine.artboard) {
            let handle = state_machine.artboard.clone();
            let state_machine =
                rive_rs::StateMachine::instantiate(&artboard, state_machine.index).unwrap();

            commands
                .entity(entity)
                .insert(RiveStateMachine(state_machine));

            artboard_entities.insert(handle.id(), entity);
        }
    }
}

fn reinstantiate_linear_animations(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<assets::Artboard>>,
    mut artboard_entities: ResMut<ArtboardEntities>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Modified { id } => {
                commands
                    .entity(*artboard_entities.get(id).unwrap())
                    .remove::<RiveLinearAnimation>()
                    .remove::<RiveStateMachine>();
            }
            AssetEvent::Removed { id } => {
                artboard_entities.remove(&id);
            }
            _ => (),
        }
    }
}

fn pass_pointer_events(
    mut scenes: Query<(
        Option<&mut RiveLinearAnimation>,
        Option<&mut RiveStateMachine>,
        &SpriteEntity,
        &Viewport,
    )>,
    sprites: Query<(&Transform, &Handle<Image>), With<Sprite>>,
    image_assets: Res<Assets<Image>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    windows: Query<&Window>,
) {
    let (camera, camera_transform) = camera.single();
    let get_world_pos = |cursor_position| {
        camera
            .viewport_to_world(camera_transform, cursor_position)
            .map(|ray| ray.origin.truncate())
    };
    let cursor_moved_events: Vec<_> = cursor_moved_events.read().collect();
    let mouse_button_input_events: Vec<_> = mouse_button_input_events.read().collect();

    for (linear_animation, state_machine, sprite_entity, viewport) in &mut scenes {
        let mut scene = get_scene_or_continue!(linear_animation, state_machine);

        let Some((transform, image_handle)) = sprite_entity
            .entity
            .map(|entity| sprites.get(entity).ok())
            .flatten()
        else {
            continue;
        };

        let image_dimensions = image_assets.get(image_handle).unwrap().size();
        let scaled_image_dimension = image_dimensions * transform.scale.truncate();
        let bounding_box =
            Rect::from_center_size(transform.translation.truncate(), scaled_image_dimension);
        let get_relative_pos =
            |world_pos| (world_pos - bounding_box.min) / transform.scale.truncate();

        for cursor_moved in &cursor_moved_events {
            if let Some(world_pos) = get_world_pos(cursor_moved.position) {
                if bounding_box.contains(world_pos) {
                    let pos = get_relative_pos(world_pos);
                    scene.pointer_move(pos.x, pos.y, &viewport);
                }
            }
        }

        for mouse_button_input in &mouse_button_input_events {
            if mouse_button_input.button != MouseButton::Left {
                continue;
            }

            if let Some(world_pos) = windows
                .get(mouse_button_input.window)
                .ok()
                .and_then(|w| w.cursor_position())
                .and_then(get_world_pos)
            {
                let pos = get_relative_pos(world_pos);
                match mouse_button_input.state {
                    ButtonState::Pressed => scene.pointer_down(pos.x, pos.y, viewport),
                    ButtonState::Released => scene.pointer_down(pos.x, pos.y, viewport),
                }
            }
        }
    }
}

fn pass_state_machine_input_events(
    mut query: Query<&mut RiveStateMachine>,
    mut input_events: EventReader<Input>,
) {
    for input in input_events.read() {
        if let Ok(state_machine) = query.get_mut(input.state_machine) {
            match input.value {
                InputValue::Bool(val) => state_machine.get_bool(&input.name).unwrap().set(val),
                InputValue::Number(val) => state_machine.get_number(&input.name).unwrap().set(val),
                InputValue::Trigger => state_machine.get_trigger(&input.name).unwrap().fire(),
            }
        }
    }
}

fn render_rive_scenes(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        Option<&mut RiveLinearAnimation>,
        Option<&mut RiveStateMachine>,
        &mut Viewport,
    )>,
) {
    let elapsed = time.delta();

    for (entity, linear_animation, state_machine, mut viewport) in &mut query {
        let mut renderer = rive_rs::Renderer::default();
        let mut scene = get_scene_or_continue!(linear_animation, state_machine);

        if scene.advance_and_maybe_draw(&mut renderer, elapsed, &mut viewport) {
            commands
                .entity(entity)
                .insert(VelloFragment(renderer.into_scene()));
        } else {
            commands.entity(entity).remove::<VelloFragment>();
        }
    }
}

fn send_generic_events(
    query: Query<(Entity, &RiveStateMachine)>,
    mut generic_events: EventWriter<GenericEvent>,
) {
    for (entity, state_machine) in &query {
        for event in state_machine.events() {
            generic_events.send(GenericEvent {
                state_machine: entity,
                name: event.name,
                delay: event.delay,
                properties: event.properties,
            })
        }
    }
}

struct VelloContextInner {
    renderer: Renderer,
    auxiliary_texture: Texture,
    has_renderered_this_frame: bool,
}

#[derive(Resource)]
struct VelloContext {
    inner: Arc<Mutex<VelloContextInner>>,
}

impl FromWorld for VelloContext {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        Self {
            inner: Arc::new(Mutex::new(VelloContextInner {
                renderer: Renderer::new(
                    device.wgpu_device(),
                    &RendererOptions {
                        surface_format: None,
                        timestamp_period: queue.get_timestamp_period(),
                        use_cpu: false,
                    },
                )
                .unwrap(),
                auxiliary_texture: device.create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d::default(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
                    view_formats: &[],
                }),
                has_renderered_this_frame: false,
            })),
        }
    }
}

fn reset_renderer(renderer: Res<VelloContext>) {
    renderer.inner.lock().unwrap().has_renderered_this_frame = false;
}

#[derive(Debug, Default)]
struct VelloNode {
    scene_entities: Vec<Entity>,
}

impl VelloNode {
    pub const NAME: &'static str = "vello";
}

impl Node for VelloNode {
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let context = world.resource::<VelloContext>().inner.clone();
        let mut context = context.lock().unwrap();

        if context.has_renderered_this_frame {
            return Ok(());
        }

        let device = render_context.render_device();
        let queue = world.resource::<RenderQueue>();
        let gpu_images = world.resource::<RenderAssets<Image>>();

        for VelloScene {
            scene,
            image_handle: image,
            ..
        } in self
            .scene_entities
            .iter()
            .copied()
            .filter_map(|e| world.get::<VelloScene>(e))
        {
            let gpu_image = gpu_images.get(image).unwrap();
            let auxiliary_texture_view = context
                .auxiliary_texture
                .create_view(&TextureViewDescriptor::default());

            context
                .renderer
                .render_to_texture(
                    device.wgpu_device(),
                    &queue,
                    &scene,
                    &auxiliary_texture_view,
                    &RenderParams {
                        base_color: vello::peniko::Color::TRANSPARENT,
                        width: gpu_image.size.x as u32,
                        height: gpu_image.size.y as u32,
                    },
                )
                .unwrap();

            let mut encoder =
                device.create_command_encoder(&CommandEncoderDescriptor { label: None });

            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &context.auxiliary_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &gpu_image.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                gpu_image.texture.size(),
            );

            queue.submit(Some(encoder.finish()));

            context.has_renderered_this_frame = true;
        }

        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        let context = world.resource::<VelloContext>().inner.clone();
        let mut context = context.lock().unwrap();

        if context.has_renderered_this_frame {
            return;
        }

        let max_size = world.query::<&VelloScene>().iter(world).fold(
            Extent3d::default(),
            |mut size, vello_scene| {
                size.width = size.width.max(vello_scene.width.next_power_of_two());
                size.height = size.height.max(vello_scene.height.next_power_of_two());

                size
            },
        );

        if context.auxiliary_texture.width() != max_size.width
            || context.auxiliary_texture.height() != max_size.height
        {
            let device = world.resource::<RenderDevice>();

            context.auxiliary_texture.destroy();
            context.auxiliary_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: max_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        }

        self.scene_entities.clear();
        self.scene_entities.extend(
            world
                .query_filtered::<Entity, With<VelloScene>>()
                .iter(world),
        );
    }
}

pub struct RivePlugin;

impl Plugin for RivePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Artboard>()
            .init_asset::<Riv>()
            .init_asset_loader::<RivLoader>()
            .init_resource::<ArtboardEntities>()
            .add_event::<Input>()
            .add_event::<GenericEvent>()
            .add_systems(
                PreUpdate,
                (
                    (insert_deafult_viewports, resize_viewports).chain(),
                    reinstantiate_linear_animations,
                    instantiate_linear_animations,
                    instantiate_state_machines,
                ),
            )
            .add_systems(
                Update,
                (
                    pass_pointer_events,
                    pass_state_machine_input_events,
                    render_rive_scenes,
                    send_generic_events,
                )
                    .chain(),
            )
            .add_plugins(ExtractComponentPlugin::<VelloScene>::default());
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<VelloContext>()
            .add_systems(Render, reset_renderer.in_set(RenderSet::Cleanup))
            .add_render_graph_node::<VelloNode>(core_2d::graph::NAME, VelloNode::NAME)
            .add_render_graph_edges(
                core_2d::graph::NAME,
                &[VelloNode::NAME, core_2d::graph::node::MAIN_PASS],
            )
            .add_render_graph_node::<VelloNode>(core_3d::graph::NAME, VelloNode::NAME)
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[VelloNode::NAME, core_3d::graph::node::START_MAIN_PASS],
            );
    }
}
