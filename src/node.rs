use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::Node,
        render_resource::{
            CommandEncoderDescriptor, Extent3d, ImageCopyTexture, Origin3d, Texture, TextureAspect,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use etagere::{euclid::Size2D, AllocId, Allocation, AtlasAllocator, Rectangle};
use vello::{kurbo::Affine, RenderParams, Renderer, RendererOptions, SceneBuilder};

use crate::components::VelloScene;

struct VelloAtlas {
    atlas_alloc: AtlasAllocator,
    alloc_ids: HashMap<Entity, AllocId>,
}

impl VelloAtlas {
    fn required_size(sizes: impl Iterator<Item = (Entity, u32, u32)>) -> u32 {
        let total_area: u32 = sizes.map(|(_, w, h)| w * h).sum();

        let theoretical_min_size = (total_area as f32).sqrt().ceil() as u32;
        let mut size = theoretical_min_size.next_power_of_two();

        if size * size < total_area * 2 {
            size *= 2;
        }

        size
    }

    pub fn new(sizes: impl Iterator<Item = (Entity, u32, u32)>) -> Self {
        let size = Self::required_size(sizes);

        Self {
            atlas_alloc: AtlasAllocator::new(Size2D::new(size as i32, size as i32)),
            alloc_ids: HashMap::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.atlas_alloc.size().width as _
    }

    pub fn height(&self) -> u32 {
        self.atlas_alloc.size().height as _
    }

    fn resize(&mut self, size: u32) {
        self.alloc_ids.clear();
        self.atlas_alloc = AtlasAllocator::new(Size2D::new(size as i32, size as i32));
    }

    fn allocate_or_resize(&mut self, width: u32, height: u32) -> AllocId {
        loop {
            if let Some(Allocation { id, .. }) = self
                .atlas_alloc
                .allocate(Size2D::new(width as i32, height as i32))
            {
                break id;
            }

            self.resize(2 * self.atlas_alloc.size().width as u32);
        }
    }

    pub fn update_size(&mut self, sizes: impl Iterator<Item = (Entity, u32, u32)>) {
        let required_size = Self::required_size(sizes);
        let current_size = self.atlas_alloc.size().width as u32;
        let current_area = current_size * current_size;

        if !(current_area / 4..=current_area).contains(&(required_size * required_size)) {
            self.resize(required_size);
        }
    }

    pub fn allocate_all(&mut self, sizes: impl Iterator<Item = (Entity, u32, u32)>) {
        for (entity, width, height) in sizes {
            if !self.alloc_ids.contains_key(&entity) {
                let id = self.allocate_or_resize(width, height);
                self.alloc_ids.insert(entity, id);
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Rectangle {
        self.atlas_alloc.get(self.alloc_ids[&entity])
    }
}

struct VelloContextInner {
    renderer: Renderer,
    atlas: Option<VelloAtlas>,
    atlas_texture: Texture,
    has_renderered_this_frame: bool,
}

#[derive(Resource)]
pub struct VelloContext {
    inner: Arc<Mutex<VelloContextInner>>,
}

impl VelloContext {
    pub fn reset_renderer(&self) {
        self.inner.lock().unwrap().has_renderered_this_frame = false;
    }
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
                atlas: None,
                atlas_texture: device.create_texture(&TextureDescriptor {
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

#[derive(Debug, Default)]
pub struct VelloNode {
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
        let atlas = context.atlas.as_ref().unwrap();

        let mut scene = vello::Scene::default();
        let mut builder = SceneBuilder::for_scene(&mut scene);
        let mut max_size = (0, 0);

        for (entity, VelloScene { fragment, .. }) in self
            .scene_entities
            .iter()
            .copied()
            .filter_map(|e| world.get::<VelloScene>(e).map(|s| (e, s)))
        {
            let rect = atlas.get(entity);
            builder.append(
                fragment,
                Some(Affine::translate((rect.min.x as f64, rect.min.y as f64))),
            );

            max_size.0 = max_size.0.max(rect.max.x as u32);
            max_size.1 = max_size.1.max(rect.max.y as u32);
        }

        let atlas_texture_view = context
            .atlas_texture
            .create_view(&TextureViewDescriptor::default());

        context
            .renderer
            .render_to_texture(
                device.wgpu_device(),
                &queue,
                &scene,
                &atlas_texture_view,
                &RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width: max_size.0,
                    height: max_size.1,
                },
            )
            .unwrap();

        let atlas = context.atlas.as_ref().unwrap();
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        for (entity, VelloScene { image_handle, .. }) in self
            .scene_entities
            .iter()
            .copied()
            .filter_map(|e| world.get::<VelloScene>(e).map(|s| (e, s)))
        {
            let gpu_image = gpu_images.get(image_handle).unwrap();
            let rect = atlas.get(entity);

            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &context.atlas_texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: rect.min.x as u32,
                        y: rect.min.y as u32,
                        ..Default::default()
                    },
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
        }

        queue.submit(Some(encoder.finish()));

        context.has_renderered_this_frame = true;

        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        let context = world.resource::<VelloContext>().inner.clone();
        let mut context = context.lock().unwrap();

        if context.has_renderered_this_frame {
            return;
        }

        let mut skip_update_size = true;
        let atlas = context.atlas.get_or_insert_with(|| {
            skip_update_size = true;
            VelloAtlas::new(
                world
                    .query::<(Entity, &VelloScene)>()
                    .iter(world)
                    .into_iter()
                    .map(|(e, s)| (e, s.width, s.height)),
            )
        });

        if !skip_update_size {
            atlas.update_size(
                world
                    .query::<(Entity, &VelloScene)>()
                    .iter(world)
                    .into_iter()
                    .map(|(e, s)| (e, s.width, s.height)),
            );
        }

        atlas.allocate_all(
            world
                .query::<(Entity, &VelloScene)>()
                .iter(world)
                .into_iter()
                .map(|(e, s)| (e, s.width, s.height)),
        );

        let atlas_size = Extent3d {
            width: atlas.width(),
            height: atlas.height(),
            ..Default::default()
        };

        if context.atlas_texture.width() != atlas_size.width
            || context.atlas_texture.height() != atlas_size.height
        {
            let device = world.resource::<RenderDevice>();

            context.atlas_texture.destroy();
            context.atlas_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: atlas_size,
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
