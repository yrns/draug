use bevy::{prelude::*, window::WindowResized};

use piet_b::{
    self as piet, glyph_rect, kurbo, FontFamily, Piet, RenderContext, Text, TextLayout,
    TextLayoutBuilder,
};

fn main() {
    App::new()
        //.add_plugins(DefaultPlugins)
        .insert_resource(WindowDescriptor {
            scale_factor_override: Some(1.0),
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
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

fn draw(mut drawn: Local<bool>, mut resized: EventReader<WindowResized>, params: piet::PietParams) {
    let resized = resized.iter().next().is_some();

    if !*drawn || resized {
        let window = params.text_params.windows.primary();
        let width = window.width() as f64;
        let height = window.height() as f64;
        let center = kurbo::Point::new(width * 0.5, height * 0.5);

        let mut piet = Piet::new(params);
        let family = FontFamily::new_unchecked("Vollkorn-Regular.ttf");
        if let Ok(layout) = piet
            .text()
            .new_text_layout("Hello, piet. ")
            .font(family, 64.0)
            .build()
        {
            let size = layout.size();
            let rect = kurbo::Rect::from_center_size(center, size);

            piet.fill(rect, &piet::Color::WHITE);

            let color = piet::Color::RED.with_alpha(0.3);
            for glyph in layout.glyphs.iter() {
                let glyph_rect = glyph_rect(glyph) + rect.origin().to_vec2();
                piet.fill(glyph_rect, &color);
            }

            piet.draw_text(&layout, rect.origin());
            *drawn = true;
        } else {
            // font is still loading
        }
    }
}
