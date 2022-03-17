use bevy::prelude::*;
use piet_b::{
    self as piet, kurbo, FontFamily, Piet, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

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

fn draw(mut drawn: Local<bool>, params: piet::PietParams) {
    if !*drawn {
        let window = params.text_params.windows.primary();
        let width = window.physical_width() as f64;
        let height = window.physical_height() as f64;
        let center = kurbo::Point::new(width * 0.5, height * 0.5);

        let mut piet = Piet::new(params);
        let family = FontFamily::new_unchecked("Vollkorn-Regular.ttf");
        if let Ok(layout) = piet
            .text()
            .new_text_layout("Hello, piet.")
            .font(family, 64.0)
            .build()
        {
            let size = layout.size();

            piet.fill(
                kurbo::Rect::from_center_size(center, size + kurbo::Size::new(20.0, 20.0)),
                &piet::Color::WHITE,
            );

            //piet.draw_text(&layout, center - (size.width * 0.5, 0.0));
            *drawn = true;
        } else {
            // font is still loading
        }
    }
}
