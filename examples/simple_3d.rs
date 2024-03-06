//! An example drawing a Rive animation (State Machine) on a 3d cube - with mouse inputs.

use bevy::{prelude::*, render::render_resource::Extent3d, window};
use rive_bevy::{RivePlugin, SceneTarget, StateMachine};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RivePlugin)
        .add_systems(Startup, setup_animation)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, window::close_on_esc)
        .run()
}

#[derive(Component)]
struct DefaultCube;

fn setup_animation(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    let cube_size = 4.0;
    let cube_handle = meshes.add(Cuboid::new(cube_size, cube_size, cube_size));

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(animation_image_handle.clone()),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    let cube_entity = commands
        .spawn((
            PbrBundle {
                mesh: cube_handle,
                material: material_handle,
                transform: Transform::from_xyz(0.0, 0.0, 1.5),
                ..default()
            },
            DefaultCube,
        ))
        .id();

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 3000.0,
            ..default()
        },
        // Light in front of the 3D camera.
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

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
        mesh: rive_bevy::MeshEntity {
            entity: Some(cube_entity),
        },
        ..default()
    });
}

fn rotate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<DefaultCube>>) {
    for mut transform in &mut query {
        transform.rotate_x(0.6 * time.delta_seconds());
        transform.rotate_y(-0.2 * time.delta_seconds());
    }
}
