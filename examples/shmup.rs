//! Shoout em up bitchass!

use std::{borrow::Cow, time::Duration};

use bevy::{
    core_pipeline::bloom::BloomSettings, prelude::*, render::render_resource::Extent3d,
    sprite::collide_aabb::collide, sprite::MaterialMesh2dBundle, window,
};

use rand::prelude::*;

use rive_bevy::{
    events, Riv, RivePlugin, RiveStateMachine, SceneTarget, SpriteEntity, StateMachine,
};
use rive_rs::scene::Scene;

// const BACKGROUND_COLOR: Color = Color::rgb(0.0, 0.0, 0.0);
const BACKGROUND_COLOR: Color = Color::rgb(0.023, 0.0, 0.102);

// SIZING
const WINDOW_SIZE: Vec2 = Vec2::new(1500.0, 1000.0);
const LEFT_BOUNT: f32 = -WINDOW_SIZE.x / 2.0;
const RIGHT_BOUNT: f32 = WINDOW_SIZE.x / 2.0;

// PLAYER
const PLAYER_SIZE: Vec3 = Vec3::new(220.0, 220.0, 0.0);
const PLAYER_IMAGE_SIZE: Vec3 = Vec3::new(220.0, 1000.0, 0.0); // Using a different size as the animation draws outside the artbooard bounds in the y-axis (clipping is disabled for the artboard)
const PLAYER_COLIDER_SIZE: Vec2 = Vec2::new(PLAYER_SIZE.x / 2.0, PLAYER_SIZE.y / 2.0);
const GAP_BETWEEN_PLAYER_AND_BOTTOM: f32 = -WINDOW_SIZE.y / 2. + PLAYER_SIZE.y / 2.0;
const PLAYER_SPEED: f32 = 500.0;

// ENEMY
const ENEMY_SIZE: Vec2 = Vec2::new(130.0, 130.0); // MAKE SIZE SMALLER TO FIT MORE ENEMIES
const ENEMY_COLIDER_SIZE: Vec2 = Vec2::new(ENEMY_SIZE.x / 2.0, ENEMY_SIZE.y / 2.0);
const GAP_BETWEEN_ENEMIES: f32 = 5.0;
const ENEMY_MOVE_TIME: f32 = 2.0; // MAKE TIME SMALLER TO MAKE ENEMIES MOVE FASTER
const ENEMY_MOVE_DISTANCE: f32 = 200.0;
const ENEMY_DESPAWN_TIME: f32 = 1.0;
const ENEMY_AREA: Vec2 = Vec2::new(WINDOW_SIZE.x / 2.0, WINDOW_SIZE.y / 2.0);

// PROJECTILES
const PROJECTILE_SIZE: Vec2 = Vec2::new(10.0, 10.0);
const PROJECTILE_SPEED: f32 = 400.0;
const PLAYER_PROJECTILE_DIRECTION: Vec2 = Vec2::new(0.0, 1.0);
const ENEMY_PROJECTILE_DIRECTION: Vec2 = Vec2::new(0.0, -1.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WINDOW_SIZE.into(),
                title: "SHMUP".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RivePlugin)
        .init_resource::<EnemyMoveTimer>()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_systems(Startup, setup)
        .add_systems(Update, window::close_on_esc)
        .add_systems(
            FixedUpdate,
            (
                collision_system,
                despawn_dead_enemies_system,
                enemies_shoot_system,
                despawn_out_of_frame_player_projectiles_system,
                despawn_out_of_frame_enemy_projectiles_system,
                despawn_out_of_frame_enemies_system,
                instantiate_projectile_system,
            ),
        )
        .add_systems(
            Update,
            (
                apply_velocity,
                player_movement_system,
                player_control_system,
                drift_player_ship_system,
                instantiate_enemies_system,
                move_enemies_over_time_system,
                move_to_target_position_system,
            ),
        )
        .run();
}

#[derive(Component)]
struct TargetPosition {
    position: Vec2,
}

#[derive(Component)]
struct Player {
    drift: f32,
    target_drift: f32,
    is_alive: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            drift: 0.0,
            target_drift: 0.0,
            is_alive: true,
        }
    }
}

#[derive(Component)]
struct Enemy {
    is_alive: bool,
}

impl Default for Enemy {
    fn default() -> Self {
        Self { is_alive: true }
    }
}

#[derive(Component)]
struct EnemyCenterSpawn {
    move_direction: MoveDirection,
}

enum MoveDirection {
    Left,
    Right,
}

#[derive(Component, Deref, DerefMut)]
struct StartingTime(u64);

#[derive(Component, Deref, DerefMut)]
struct Collider {
    size: Vec2,
}

#[derive(Component)]

struct PlayerProjectile;

#[derive(Component)]

struct EnemyProjectile;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Resource, Deref, DerefMut)]
struct EnemyMoveTimer(Timer);

impl Default for EnemyMoveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(ENEMY_MOVE_TIME, TimerMode::Repeating))
    }
}

#[derive(Component, Deref, DerefMut)]
struct EnemyDespawnTimer(Timer); // Despawn's enemy after time

impl Default for EnemyDespawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(ENEMY_DESPAWN_TIME, TimerMode::Once))
    }
}

#[derive(Component, Deref, DerefMut)]
struct EnemyShootTimer(Timer);

impl Default for EnemyShootTimer {
    fn default() -> Self {
        let random_time = rand::thread_rng().gen_range(3..10) as f32;
        Self(Timer::from_seconds(random_time, TimerMode::Once))
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings::OLD_SCHOOL,
    ));

    // Background
    spawn_background(&mut commands, meshes, materials);

    // Player
    {
        let player_y = GAP_BETWEEN_PLAYER_AND_BOTTOM;
        let mut player_image = Image::default();

        player_image.resize(Extent3d {
            width: (PLAYER_IMAGE_SIZE.x as u32) * 2,
            height: (PLAYER_IMAGE_SIZE.y as u32) * 2,
            ..default()
        });

        let rect_image_handle = images.add(player_image);

        let sm = StateMachine {
            riv: asset_server.load("shmup/ship.riv"),
            ..default()
        };

        let player_entity = commands
            .spawn((
                SpriteBundle {
                    texture: rect_image_handle.clone(),
                    transform: Transform::from_scale(Vec3::new(0.5, 0.5, 1.0))
                        .with_translation(Vec3::new(0.0, player_y, 0.0)),
                    ..default()
                },
                Player::default(),
                Collider {
                    size: PLAYER_COLIDER_SIZE,
                },
                sm,
            ))
            .id();

        commands.spawn(SceneTarget {
            image: rect_image_handle,
            // Adding the sprite here enables mouse input being passed to the Scene.
            sprite: SpriteEntity {
                entity: Some(player_entity),
            },
            ..default()
        });
    }

    let center_y = (WINDOW_SIZE.y - ENEMY_AREA.y) / 2.0;

    commands.spawn((
        Transform {
            translation: Vec3::new(0.0, center_y, 0.0),
            scale: Vec3::new(10.0, 10.0, 1.0),
            ..default()
        },
        EnemyCenterSpawn {
            move_direction: MoveDirection::Right,
        },
    ));

    // Given the space available, compute how many rows and columns of enemies we can fit.
    let n_columns = (ENEMY_AREA.x / (ENEMY_SIZE.x + GAP_BETWEEN_ENEMIES)).floor() as usize;
    let n_rows = (ENEMY_AREA.y / (ENEMY_SIZE.y + GAP_BETWEEN_ENEMIES)).floor() as usize;
    let n_vertical_gaps = n_columns - 1;

    let center_of_enenmies = 0.0;
    let left_edge_of_enemies = center_of_enenmies
        - (n_columns as f32 / 2.0 * ENEMY_SIZE.x)
        - n_vertical_gaps as f32 / 2.0 * GAP_BETWEEN_ENEMIES;

    let offset_x = left_edge_of_enemies + ENEMY_SIZE.x / 2.0;
    let offset_y = ENEMY_SIZE.y / 2. + center_y;

    let mut enemy_image = Image::default();

    enemy_image.resize(Extent3d {
        width: (ENEMY_SIZE.x as u32) * 2,
        height: (ENEMY_SIZE.y as u32) * 2,
        ..default()
    });

    for row in 0..n_rows {
        for column in 0..n_columns {
            let enemy_position = Vec2::new(
                offset_x + column as f32 * (ENEMY_SIZE.x + GAP_BETWEEN_ENEMIES),
                offset_y + row as f32 * (ENEMY_SIZE.y + GAP_BETWEEN_ENEMIES),
            );

            let enemy_image_handle = images.add(enemy_image.clone());

            let state_machine = StateMachine {
                riv: load_random_bug(&asset_server),
                ..default()
            };

            // enemy spawn
            let sprite_entity = commands
                .spawn((
                    SpriteBundle {
                        texture: enemy_image_handle.clone(),
                        transform: Transform::from_scale(Vec3::new(0.5, 0.5, 1.0))
                            .with_translation(enemy_position.extend(0.0)),
                        ..default()
                    },
                    Enemy::default(),
                    TargetPosition {
                        position: enemy_position,
                    },
                    Collider {
                        size: ENEMY_COLIDER_SIZE,
                    },
                    EnemyShootTimer::default(),
                    state_machine,
                ))
                .id();

            commands.spawn(SceneTarget {
                image: enemy_image_handle,
                sprite: SpriteEntity {
                    entity: Some(sprite_entity),
                },
                ..default()
            });
        }
    }
}

fn player_movement_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player)>,
    time_step: Res<Time>,
) {
    let (mut player_transform, mut player) = query.single_mut();

    if !player.is_alive {
        return;
    }

    let mut direction = 0.0;
    player.target_drift = 0.0;

    if keyboard_input.pressed(KeyCode::Left) {
        direction -= 1.0;
        player.target_drift = -100.0;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        direction += 1.0;
        player.target_drift = 100.0;
    }

    // Calculate the new player position based on the input.
    let new_player_position =
        player_transform.translation.x + direction * PLAYER_SPEED * time_step.delta().as_secs_f32();

    // Update player position
    // making sure it doesn't cause the player to go out of bounds.
    let left_bound = LEFT_BOUNT + PLAYER_SIZE.x / 2.0;
    let right_bound = RIGHT_BOUNT - PLAYER_SIZE.x / 2.0;
    player_transform.translation.x = new_player_position.clamp(left_bound, right_bound);
}

// Randomize the enemy animation start time.
fn instantiate_enemies_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut RiveStateMachine), (Without<StartingTime>, Without<Player>)>,
) {
    for (entity, mut sm) in &mut query {
        let random_time = rand::thread_rng().gen_range(0..1000) * 5;
        sm.as_mut().advance_and_apply(Duration::from_millis(0)); // TODO: this is a bug - need to call 0  before setting a starting time
        sm.as_mut()
            .advance_and_apply(Duration::from_millis(random_time));
        commands.entity(entity).insert(StartingTime(random_time));
    }
}

fn instantiate_projectile_system(mut query: Query<&mut RiveStateMachine, Added<EnemyProjectile>>) {
    // Set projectile state machine input to isEnemyProjectile = true
    // which changes the color of the projectile. Default is the player color projectile.
    for sm in &mut query {
        sm.get_bool("isEnemyProjectile").unwrap().set(true);
    }
}

fn player_control_system(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    query: Query<(Entity, &Transform, &Player)>,
    mut input_events: EventWriter<events::Input>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let (entity, transform, player) = query.single();
        if !player.is_alive {
            return;
        }
        input_events.send(events::Input {
            state_machine: entity,
            name: Cow::Owned("shoot".to_string()),
            value: events::InputValue::Trigger,
        });

        let mut projectile_image = Image::default();

        projectile_image.resize(Extent3d {
            width: (PROJECTILE_SIZE.x as u32) * 3,
            height: (PROJECTILE_SIZE.y as u32) * 3,
            ..default()
        });

        let rect_image_handle = images.add(projectile_image);

        let state_machine = StateMachine {
            riv: asset_server.load("shmup/projectile.riv"),
            ..default()
        };

        let projectile_entity = commands
            .spawn((
                SpriteBundle {
                    texture: rect_image_handle.clone(),
                    transform: Transform {
                        scale: Vec3::new(1.0, 1.0, 1.0),
                        translation: transform.translation
                            + Vec3::new(0.0, PLAYER_SIZE.y / 2.0, 0.0),
                        ..default()
                    },
                    sprite: Sprite {
                        color: Color::rgb(4.0, 4.0, 6.0), // 4. Put something bright in a dark environment to see the effect
                        custom_size: Some(PROJECTILE_SIZE * 2.0),
                        ..default()
                    },
                    ..default()
                },
                PlayerProjectile,
                Collider {
                    size: PROJECTILE_SIZE,
                },
                Velocity(PLAYER_PROJECTILE_DIRECTION * PROJECTILE_SPEED),
                state_machine,
            ))
            .id();

        commands.spawn(SceneTarget {
            image: rect_image_handle,
            // Adding the sprite here enables mouse input being passed to the Scene.
            sprite: SpriteEntity {
                entity: Some(projectile_entity),
            },
            ..default()
        });
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time_step: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time_step.delta().as_secs_f32();
        transform.translation.y += velocity.y * time_step.delta().as_secs_f32();
    }
}

fn collision_system(
    mut commands: Commands,
    mut enemy_query: Query<(
        Entity,
        &Transform,
        &Collider,
        &mut Enemy,
        &mut TargetPosition,
    )>,
    player_projectiles: Query<(Entity, &Transform), With<PlayerProjectile>>,
    enemy_projectiles: Query<(Entity, &Transform), With<EnemyProjectile>>,
    mut player_query: Query<(Entity, &Transform, &Collider, &mut Player)>,
    mut input_events: EventWriter<events::Input>,
) {
    // Enemy projectiles on player
    for (projectile_entity, transform) in &enemy_projectiles {
        for (player_entity, player_transform, collider, mut player) in player_query.iter_mut() {
            let collision = collide(
                transform.translation,
                PROJECTILE_SIZE,
                player_transform.translation,
                collider.size,
            );

            if collision.is_some() {
                if !player.is_alive {
                    continue; // player already destroyed, waiting to despawn
                }
                player.is_alive = false;

                commands.entity(projectile_entity).despawn();

                // Send explosition input to player state machine.
                input_events.send(events::Input {
                    state_machine: player_entity,
                    name: Cow::Owned("explosion".to_string()),
                    value: events::InputValue::Trigger,
                });
            }
        }
    }

    // Player projectiles on enemies
    for (projectile_entity, transform) in &player_projectiles {
        for (enemy_entity, enemy_transform, collider, mut enemy, mut target_position) in
            enemy_query.iter_mut()
        {
            let collision = collide(
                transform.translation,
                PROJECTILE_SIZE,
                enemy_transform.translation,
                collider.size,
            );

            if collision.is_some() {
                if !enemy.is_alive {
                    continue; // enemy already destroyed, waiting to despawn
                }
                enemy.is_alive = false;

                commands.entity(projectile_entity).despawn();

                target_position.position += Vec2::new(0.0, -300.0);
                commands
                    .entity(enemy_entity)
                    .insert(EnemyDespawnTimer::default());

                // Set enemy state machine input to isAlive = false.
                input_events.send(events::Input {
                    state_machine: enemy_entity,
                    name: Cow::Owned("isAlive".to_string()),
                    value: events::InputValue::Bool(false),
                });
            }
        }
    }
}

fn enemies_shoot_system(
    mut commands: Commands,
    mut enemy_query: Query<(Entity, &mut EnemyShootTimer, &Transform)>,
    mut images: ResMut<Assets<Image>>,
    mut input_events: EventWriter<events::Input>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (enemy_entity, mut enemy_shoot_timer, transform) in &mut enemy_query.iter_mut() {
        enemy_shoot_timer.tick(time.delta());

        if enemy_shoot_timer.finished() {
            let random_time = rand::thread_rng().gen_range(2..10) as f32;
            enemy_shoot_timer.0 = Timer::from_seconds(random_time, TimerMode::Once);
            enemy_shoot_timer.reset();

            let mut projectile_image = Image::default();

            projectile_image.resize(Extent3d {
                width: (PROJECTILE_SIZE.x as u32) * 3,
                height: (PROJECTILE_SIZE.y as u32) * 3,
                ..default()
            });

            let rect_image_handle = images.add(projectile_image);

            let state_machine = StateMachine {
                riv: asset_server.load("shmup/projectile.riv"),
                ..default()
            };

            let projectile = commands
                .spawn((
                    SpriteBundle {
                        texture: rect_image_handle.clone(),
                        transform: Transform {
                            scale: Vec3::new(1.0, 1.0, 1.0),

                            translation: transform.translation,
                            ..default()
                        },
                        sprite: Sprite {
                            color: Color::rgb(4.0, 4.0, 6.0), // 4. Put something bright in a dark environment to see the effect
                            custom_size: Some(PROJECTILE_SIZE * 2.0),
                            ..default()
                        },
                        ..default()
                    },
                    EnemyProjectile,
                    Collider {
                        size: PROJECTILE_SIZE,
                    },
                    Velocity(ENEMY_PROJECTILE_DIRECTION * PROJECTILE_SPEED),
                    state_machine,
                ))
                .id();

            commands.spawn(SceneTarget {
                image: rect_image_handle,
                // Adding the sprite here enables mouse input being passed to the Scene.
                sprite: SpriteEntity {
                    entity: Some(projectile),
                },
                ..default()
            });

            // Play shoot animation on enemy state machine.
            input_events.send(events::Input {
                state_machine: enemy_entity,
                name: Cow::Owned("Shoot".to_string()),
                value: events::InputValue::Trigger,
            });
        }
    }
}

fn move_enemies_over_time_system(
    mut enemies: Query<(&mut TargetPosition, &Enemy)>,
    mut enemies_center: Query<(&mut Transform, &mut EnemyCenterSpawn), Without<Enemy>>,
    mut enemy_move_timer: ResMut<EnemyMoveTimer>,
    time: Res<Time>,
) {
    enemy_move_timer.tick(time.delta());

    let (mut transform, mut spawn) = enemies_center.single_mut();
    let moveable_space = WINDOW_SIZE - ENEMY_AREA;

    if enemy_move_timer.finished() {
        let mut new_position_offset: Vec2 = Vec2::new(0.0, -ENEMY_MOVE_DISTANCE);
        let right_edge_distance = (moveable_space.x / 2.0) - transform.translation.x;
        let left_edge_distance = (moveable_space.x / 2.0) + transform.translation.x;

        match spawn.move_direction {
            MoveDirection::Left => (if left_edge_distance > ENEMY_SIZE.x {
                transform.translation.x -= ENEMY_MOVE_DISTANCE;
                new_position_offset = Vec2::new(-ENEMY_MOVE_DISTANCE, 0.0);
            } else {
                spawn.move_direction = MoveDirection::Right;
            },),
            MoveDirection::Right => (if right_edge_distance > ENEMY_SIZE.x {
                transform.translation.x += ENEMY_MOVE_DISTANCE;
                new_position_offset = Vec2::new(ENEMY_MOVE_DISTANCE, 0.0);
            } else {
                spawn.move_direction = MoveDirection::Left;
            },),
        };

        for (mut target_position, enemy) in &mut enemies {
            if enemy.is_alive {
                target_position.position += new_position_offset;
            }
        }
    }
}

fn move_to_target_position_system(
    mut query: Query<(&mut Transform, &TargetPosition)>,
    time: Res<Time>,
) {
    for (mut transform, target_position) in &mut query {
        transform.translation = transform
            .translation
            .lerp(target_position.position.extend(0.0), time.delta_seconds());
    }
}

fn drift_player_ship_system(
    mut query: Query<(Entity, &mut Player)>,
    mut input_events: EventWriter<events::Input>,
) {
    let (entity, mut player) = query.single_mut();

    let mut current_drift = lerp(player.drift, player.target_drift, 0.1);

    current_drift = current_drift.clamp(-100.0, 100.0);

    player.drift = current_drift;

    // Send Rive input event to update the state machine's drift input.
    input_events.send(events::Input {
        state_machine: entity,
        name: Cow::Owned("drift".to_string()),
        value: events::InputValue::Number(player.drift),
    });
}

fn despawn_dead_enemies_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut EnemyDespawnTimer)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_out_of_frame_player_projectiles_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform), With<PlayerProjectile>>,
) {
    for (entity, transform) in &mut query.iter_mut() {
        if transform.translation.y < -WINDOW_SIZE.y / 2.0
            || transform.translation.y > WINDOW_SIZE.y / 2.0
        {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_out_of_frame_enemy_projectiles_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform), With<EnemyProjectile>>,
) {
    for (entity, transform) in &mut query.iter_mut() {
        if transform.translation.y < -WINDOW_SIZE.y / 2.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_out_of_frame_enemies_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (entity, transform) in &mut query.iter_mut() {
        if transform.translation.y < -(WINDOW_SIZE.y / 2.0 + ENEMY_SIZE.y) {
            commands.entity(entity).despawn();
        }
    }
}

// SPAWN

fn spawn_background(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(200.0).into()).into(),
            material: materials.add(ColorMaterial::from(Color::rgb(7.5, 5.0, 7.5))),
            transform: Transform::from_translation(Vec3::new(750.0, 500.0, -5.0)),
            ..default()
        },
        Velocity(Vec2::new(0.0, -4.0)),
    ));

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(190.0).into()).into(),
            material: materials.add(ColorMaterial::from(Color::rgb(1.0, 6.0, 7.0))),
            transform: Transform::from_translation(Vec3::new(-900.0, -500.0, -5.0)),
            ..default()
        },
        Velocity(Vec2::new(0.0, -2.0)),
    ));

    let colors: Vec<Color> = vec![
        Color::rgb(7.5, 5.0, 7.5),
        Color::rgb(5.0, 7.5, 7.5),
        Color::rgb(7.5, 7.5, 5.0),
        Color::rgb(1.0, 1.0, 3.0),
    ];

    (0..100).for_each(|_| {
        let mut rng = thread_rng();
        let x: f32 = rng.gen_range(-1.0..1.0);
        let y: f32 = rng.gen_range(-1.0..1.0);
        let color: Color = *colors.choose(&mut rand::thread_rng()).unwrap();
        let size = rand::thread_rng().gen_range(0.1..2.5);

        commands.spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(size).into()).into(),
            material: materials.add(ColorMaterial::from(color)),
            transform: Transform::from_translation(Vec3::new(
                WINDOW_SIZE.x / 2. * x,
                WINDOW_SIZE.y / 2. * y,
                -5.0,
            )),
            ..default()
        });
    });
}

// UTILS

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn load_random_bug(asset_server: &Res<AssetServer>) -> Handle<Riv> {
    let val = rand::thread_rng().gen_range(1..4);

    let path = format!("shmup/bug_{val}.riv");
    asset_server.load(path)
}
