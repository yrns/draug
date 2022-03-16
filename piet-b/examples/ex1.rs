use bevy::prelude::*;
use piet_b::{self as piet, kurbo, FontFamily, Piet, RenderContext, Text, TextLayoutBuilder};

fn main() {
    App::new()
        //.add_plugins(DefaultPlugins)
        .add_plugin(bevy::log::LogPlugin::default())
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
        .add_plugin(piet::PietPlugin::default())
        .add_startup_system(setup)
        .add_system(draw)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
}

fn draw(mut drawn: Local<bool>, mut params: piet::PietParams) {
    if !*drawn {
        let mut piet = Piet::new(params);
        let color = piet::Color::PURPLE;
        let family = FontFamily::new_unchecked("Vollkorn-Regular.ttf");
        //piet.fill(kurbo::Rect::new(0.0, 0.0, 200.0, 100.0), &color);
        if let Ok(layout) = piet
            .text()
            .new_text_layout("Hello, piet.")
            .font(family, 24.0)
            .build()
        {
            //piet.draw_text();
            *drawn = true;
            dbg!("draw!");
        } else {
            dbg!("loading?");
        }
    }
}
