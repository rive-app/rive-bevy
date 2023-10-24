//! Demonstrates how to use transparency in 3D.
//! Shows the effects of different blend modes.
//! The `fade_transparency` system smoothly changes the transparency over time.

use std::borrow::Cow;

use bevy::{
    core_pipeline::bloom::{BloomCompositeMode, BloomPrefilterSettings, BloomSettings},
    pbr::NotShadowCaster,
    prelude::*,
    render::render_resource::Extent3d,
    window,
};

use rive_bevy::{MeshEntity, RivePlugin, SceneTarget, StateMachine};

fn main() {
    App::new()
        .insert_resource(Msaa::Sample8)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(RivePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, window::close_on_esc)
        .add_systems(Update, camera_control_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.0).into()),
        material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
        transform: Transform::from_xyz(0.0, -5.0, 0.0),
        ..default()
    });

    let mut rive_image = Image::default();

    rive_image.resize(Extent3d {
        width: 1920 * 2,
        height: 1080 * 2,
        ..default()
    });

    let rive_iamge_handle = images.add(rive_image);

    let plane_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(19.20, 10.80))));

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(rive_iamge_handle.clone()),
        reflectance: 1.0,
        perceptual_roughness: 0.0,
        metallic: 0.5,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let plane_entity = commands
        .spawn(PbrBundle {
            mesh: plane_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.5, 0.05),
            ..default()
        })
        .id();

    commands
        .spawn(StateMachine {
            riv: asset_server.load("sophia_iii_clear.riv"),
            artboard_handle: rive_rs::Handle::Name(Cow::Owned("SOPHIA III HUD".to_string())),
            ..default()
        })
        .insert(SceneTarget {
            image: rive_iamge_handle,
            // Adding the mesh here enables mouse input being passed to the Scene.
            mesh: MeshEntity {
                entity: Some(plane_entity),
            },
            ..default()
        });

    // opaque sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(
            Mesh::try_from(shape::Icosphere {
                radius: 2.,
                subdivisions: 3,
            })
            .unwrap(),
        ),
        material: materials.add(Color::rgb(0.7, 0.2, 0.1).into()),
        transform: Transform::from_xyz(0.0, 0.5, -5.5),
        ..default()
    });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 10000.0,
            range: 40.0,
            // radius: 10.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::new(0., 0., 1.), Vec3::X),
        ..default()
    });

    // sky
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::default())),
            material: materials.add(StandardMaterial {
                base_color: Color::hex("333333").unwrap(),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(200.0)),
            ..default()
        },
        NotShadowCaster,
    ));

    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom
                ..default()
            },
            transform: Transform::from_xyz(-4.0, 1.0, 15.0)
                .looking_at(Vec3::new(0., 0., 0.), Vec3::Y),

            ..default()
        },
        BloomSettings {
            intensity: 0.2,
            low_frequency_boost: 0.7,
            low_frequency_boost_curvature: 0.95,
            high_pass_frequency: 1.0,
            prefilter_settings: BloomPrefilterSettings {
                threshold: 0.6,
                threshold_softness: 0.2,
            },
            composite_mode: BloomCompositeMode::Additive,
        },
    ));
}

fn camera_control_system(
    mut camera: Query<(&mut Camera, &mut Transform, &GlobalTransform), With<Camera3d>>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    let (mut camera, mut camera_transform, _) = camera.single_mut();

    if input.just_pressed(KeyCode::H) {
        camera.hdr = !camera.hdr;
    }

    let rotation = if input.pressed(KeyCode::Left) {
        time.delta_seconds()
    } else if input.pressed(KeyCode::Right) {
        -time.delta_seconds()
    } else {
        0.0
    };

    let movement = if input.pressed(KeyCode::Up) {
        -time.delta_seconds()
    } else if input.pressed(KeyCode::Down) {
        time.delta_seconds()
    } else {
        0.0
    };

    camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(rotation));
    camera_transform.translation += Vec3::new(0.0, 0.0, movement * 10.0);
}
