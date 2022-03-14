use bevy::prelude::*;
use piet::{Color, RenderContext};
use piet_b::{kurbo::Rect, *};

fn main() {
    App::new()
        //.add_plugins(DefaultPlugins)
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default())
        .add_plugin(bevy::sprite::SpritePlugin::default())
        .add_plugin(bevy::text::TextPlugin::default())
        // this replaces UiPlugin, can't have both
        .add_plugin(PietPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(mut params: PietParams) {
    params.commands.spawn_bundle(UiCameraBundle::default());

    let mut piet = Piet::new(params);
    piet.fill(Rect::new(0.0, 0.0, 200.0, 100.0), &Color::PURPLE);
}
