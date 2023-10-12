use std::{collections::HashMap, sync::Arc};

use bevy::{
    core_pipeline::{core_2d, core_3d},
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, render_graph::RenderGraphApp, Render, RenderApp,
        RenderSet,
    },
};
use rive_rs::Instantiate;

use crate::{
    assets::{self, Artboard, Riv, RivLoader},
    components::{
        LinearAnimation, RiveLinearAnimation, RiveStateMachine, SpriteEntity, StateMachine,
        VelloFragment, VelloScene, Viewport,
    },
    events::{GenericEvent, Input, InputValue},
    node,
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
                .insert(VelloFragment(Arc::new(renderer.into_scene())));
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

fn reset_renderer(context: Res<node::VelloContext>) {
    context.reset_renderer();
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
            .init_resource::<node::VelloContext>()
            .add_systems(Render, reset_renderer.in_set(RenderSet::Cleanup))
            .add_render_graph_node::<node::VelloNode>(core_2d::graph::NAME, node::VelloNode::NAME)
            .add_render_graph_edges(
                core_2d::graph::NAME,
                &[node::VelloNode::NAME, core_2d::graph::node::MAIN_PASS],
            )
            .add_render_graph_node::<node::VelloNode>(core_3d::graph::NAME, node::VelloNode::NAME)
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[node::VelloNode::NAME, core_3d::graph::node::START_MAIN_PASS],
            );
    }
}
