use bevy::{prelude::*, render::extract_component::ExtractComponent};
use vello::{SceneBuilder, SceneFragment};

use crate::assets::Artboard;

#[derive(Clone, Component, Debug, Default)]
pub struct LinearAnimation {
    pub artboard: Handle<Artboard>,
    pub index: Option<usize>,
    pub sprite_entity: Option<Entity>,
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct RiveLinearAnimation(pub rive_rs::LinearAnimation);

#[derive(Clone, Component, Debug, Default)]
pub struct StateMachine {
    pub artboard: Handle<Artboard>,
    pub index: Option<usize>,
    pub sprite_entity: Option<Entity>,
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct RiveStateMachine(pub rive_rs::StateMachine);

#[derive(Clone, Component, Debug, Default, Deref, DerefMut)]
pub struct Viewport(pub rive_rs::Viewport);

#[derive(Clone, Component, Debug, Default, Deref)]
pub struct SpriteEntity {
    pub entity: Option<Entity>,
}

#[derive(Bundle, Debug, Default)]
pub struct SceneTarget {
    pub image: Handle<Image>,
    pub sprite: SpriteEntity,
}

#[derive(Component, Deref)]
pub(crate) struct VelloFragment(pub SceneFragment);

#[derive(Component)]
pub(crate) struct VelloScene(pub vello::Scene, pub Handle<Image>);

impl ExtractComponent for VelloScene {
    type Query = (&'static VelloFragment, &'static Handle<Image>);

    type Filter = ();

    type Out = Self;

    fn extract_component(
        (fragment, image): bevy::ecs::query::QueryItem<'_, Self::Query>,
    ) -> Option<Self> {
        let mut scene = vello::Scene::default();
        let mut builder = SceneBuilder::for_scene(&mut scene);
        builder.append(&fragment.0, None);

        Some(Self(scene, image.clone()))
    }
}
