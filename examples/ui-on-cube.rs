use bevy::{prelude::*, render::render_resource::Extent3d, window};
use rive_bevy::{LinearAnimation, RivePlugin, SceneTarget, SpriteEntity, StateMachine};

#[derive(Component)]
struct DefaultCube;

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut cube_image = Image::default();

    // We fill the CPU image with 0s before sending it to the GPU.
    cube_image.resize(Extent3d {
        width: 512,
        height: 512,
        ..default()
    });

    // This creates two separate assets.
    let cube_image_handle = images.add(cube_image.clone());
    let rect_image_handle = images.add(cube_image.clone());

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            // A bit brighter to make Marty clearly visible.
            intensity: 3000000.0,
            ..default()
        },
        // Light in front of the 3D camera.
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    let cube_size = 4.0;
    let cube_handle = meshes.add(Cuboid::new(cube_size, cube_size, cube_size));

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(cube_image_handle.clone()),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.0, 1.5),
            ..default()
        },
        DefaultCube,
    ));

    commands.spawn(Camera3dBundle {
        camera: Camera {
            order: 0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands
        .spawn(LinearAnimation {
            riv: asset_server.load("marty.riv"),
            ..default()
        })
        .insert(SceneTarget {
            // Rive Scene will render into the same texture as the cube is using.
            image: cube_image_handle,
            ..default()
        });

    commands.spawn(Camera2dBundle {
        camera: Camera {
            // UI (2D) camera will be rendered on top of the 3D world.
            order: 1,
            ..default()
        },
        camera_2d: Camera2d {
            // We don't want to clear the 3D objects behind our UI.
            // clear_color: ClearColorConfig::None,
        },
        ..default()
    });

    let sprite_entity = commands
        .spawn(SpriteBundle {
            texture: rect_image_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(0.5))
                .with_translation(Vec3::new(-250.0, 0.0, 0.0)),
            ..default()
        })
        .id();

    commands
        .spawn(StateMachine {
            riv: asset_server.load("rating-animation.riv"),
            ..default()
        })
        .insert(SceneTarget {
            image: rect_image_handle,
            // Adding the sprite here enables mouse input being passed to the Scene.
            sprite: SpriteEntity {
                entity: Some(sprite_entity),
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin::default()))
        .add_plugins(RivePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, window::close_on_esc)
        .add_systems(Update, rotate_cube)
        .run()
}
