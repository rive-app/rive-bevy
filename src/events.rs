use std::{borrow::Cow, collections::BTreeMap, time::Duration};

use bevy::prelude::*;
use rive_rs::state_machine::Property;

#[derive(Clone, Debug)]
pub enum InputValue {
    Bool(bool),
    Number(f32),
    Trigger,
}

#[derive(Clone, Debug, Event)]
pub struct Input {
    pub state_machine: Entity,
    pub name: Cow<'static, str>,
    pub value: InputValue,
}

#[derive(Clone, Debug, Event)]
pub struct GenericEvent {
    pub state_machine: Entity,
    pub name: String,
    pub delay: Duration,
    pub properties: BTreeMap<String, Property>,
}
