#![allow(clippy::type_complexity)]

mod assets;
mod components;
pub mod events;
mod node;
mod plugin;
mod pointer_events;

// Re-export rive-rs
pub use rive_rs;

pub use crate::{
    assets::Riv,
    components::{
        LinearAnimation, MeshEntity, RiveLinearAnimation, RiveStateMachine, SceneTarget,
        SpriteEntity, StateMachine,
    },
    events::GenericEvent,
    plugin::RivePlugin,
    rive_rs::Handle,
};
