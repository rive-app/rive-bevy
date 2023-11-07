use bevy::{prelude::*, render::render_resource::Extent3d, window};
use rive_bevy::{RivePlugin, SceneTarget, SpriteEntity, StateMachine};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RivePlugin)
        .add_systems(Startup, setup_animation)
        .add_systems(Update, window::close_on_esc)
        .run()
}

fn setup_animation(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut animation_image = Image::default();

    // We fill the CPU image with 0s before sending it to the GPU.
    animation_image.resize(Extent3d {
        width: 512,
        height: 512,
        ..default()
    });

    let animation_image_handle = images.add(animation_image.clone());

    commands.spawn(Camera2dBundle { ..default() });

    let sprite_entity = commands
        .spawn(SpriteBundle {
            texture: animation_image_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(1.0)),
            ..default()
        })
        .id();

    let linear_animation = StateMachine {
        riv: asset_server.load("rating-animation.riv"),
        // Optionally provide state machine name to load
        handle: rive_bevy::Handle::Name("State Machine 1".into()),
        // Optionally provide artboard name to load
        artboard_handle: rive_bevy::Handle::Name("New Artboard".into()),
        ..default()
    };

    commands.spawn(linear_animation).insert(SceneTarget {
        image: animation_image_handle,
        // Adding the sprite here enables mouse input being passed to the Scene.
        sprite: SpriteEntity {
            entity: Some(sprite_entity),
        },
        ..default()
    });
}
