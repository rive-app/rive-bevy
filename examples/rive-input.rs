//! An example showcasing how to manipulate Rive state machine inputs and text.

use bevy::{prelude::*, render::render_resource::Extent3d, window};
use rive_bevy::{
    events::{self, InputValue},
    RivePlugin, RiveStateMachine, SceneTarget, SpriteEntity, StateMachine,
};
use rive_rs::components::TextValueRun;

const BACKGROUND_COLOR: Color = Color::rgb(0., 0., 0.);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin::default()))
        .add_plugins(RivePlugin)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_systems(Startup, (setup_animation, setup_text))
        .add_systems(Update, window::close_on_esc)
        .add_systems(
            Update,
            (update_state_machine_system, update_rive_text_system),
        )
        .run()
}

fn setup_animation(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let mut animation_image = Image::default();

    animation_image.resize(Extent3d {
        width: 2000,
        height: 1700,
        ..default()
    });

    let animation_image_handle = images.add(animation_image.clone());

    commands.spawn(Camera2dBundle {
        camera: Camera { ..default() },
        ..default()
    });

    let sprite_entity = commands
        .spawn(SpriteBundle {
            texture: animation_image_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(0.5))
                .with_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        })
        .id();

    let state_machine = StateMachine {
        riv: asset_server.load("circle-fui.riv"),
        // artboard_handle: rive_bevy::Handle::Name("StateMachine".into()), // specify the artboard by name
        ..default()
    };

    commands.spawn(state_machine).insert(SceneTarget {
        image: animation_image_handle,
        // Adding the sprite here enables mouse input being passed to the Scene.
        sprite: SpriteEntity {
            entity: Some(sprite_entity),
        },
        ..default()
    });
}

fn setup_text(mut commands: Commands) {
    commands.spawn(
        TextBundle::from_sections([TextSection::new(
            "Update Rive state machine inputs and text",
            TextStyle {
                font_size: 22.0,
                color: Color::WHITE,
                ..default()
            },
        )])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );

    commands.spawn(
        TextBundle::from_sections([TextSection::new(
            "Press `Return` to toggle, then type to change text...",
            TextStyle {
                font_size: 22.0,
                color: Color::WHITE,
                ..default()
            },
        )])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(52.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

fn update_state_machine_system(
    kbd: Res<Input<KeyCode>>,
    mut query: Query<(Entity, &mut RiveStateMachine)>,
    mut input_events: EventWriter<events::Input>,
) {
    if kbd.just_pressed(KeyCode::Return) {
        // Get the State Machine and its Entity
        let (entity, state_machine) = query.single_mut();

        // Read the current value of an input
        let center_hover_current = state_machine.get_bool("centerHover").unwrap().get();

        // Send a new value to the input using Bevy events.
        {
            input_events.send(events::Input {
                state_machine: entity,
                name: "centerHover".into(),
                value: InputValue::Bool(!center_hover_current),
            });
        }

        // Alternatively we can use the raw API and send the value directly to the Rive C++ API.
        // Comment the above Bevy event and uncomment the below.
        {
            // state_machine
            //     .get_bool("centerHover")
            //     .unwrap()
            //     .set(!center_hover_current);
        }
    }
}

fn update_rive_text_system(
    kbd: Res<Input<KeyCode>>,
    mut query: Query<&mut RiveStateMachine>,
    mut string: Local<String>,
    mut evr_char: EventReader<ReceivedCharacter>,
) {
    // On toggle, clear the string.
    if kbd.just_pressed(KeyCode::Return) {
        string.clear();
        return;
    }

    let mut did_change = false;
    if kbd.just_pressed(KeyCode::Back) {
        did_change = true;
        string.pop();
    }
    for ev in evr_char.read() {
        // Ignore control (special) characters.
        if !ev.char.is_control() {
            string.push(ev.char);
            did_change = true;
            info!("{}", string.as_str());
        }
    }

    // Update our Rive text if the string changed.
    if did_change {
        let state_machine = query.single_mut();

        if !state_machine.get_bool("centerHover").unwrap().get() {
            return;
        }

        let mut artboard = state_machine.artboard();

        let mut text: TextValueRun = artboard
            .components()
            .find(|comp| comp.name() == "Sector")
            .unwrap()
            .try_into()
            .unwrap();

        let mut formatted_value: String = string.to_owned();
        formatted_value.push_str(" : ");

        text.set_text(&formatted_value);
    }
}
