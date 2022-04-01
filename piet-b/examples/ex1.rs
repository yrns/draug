use bevy::{prelude::*, window::WindowResized};

use piet_b::{
    self as piet, kurbo, FontFamily, Piet, PietImage, PietTextLayout, RenderContext, Text,
    TextLayout, TextLayoutBuilder,
};

fn main() {
    App::new()
        //.add_plugins(DefaultPlugins)
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(UiCameraBundle::default());
    commands.insert_resource(PietImage::from(asset_server.load("hatch.png")));
}

fn draw(
    image: Res<PietImage>,
    mut layout: Local<Option<PietTextLayout>>,
    mut resized: EventReader<WindowResized>,
    mut cursor_moved: EventReader<CursorMoved>,
    mut hit_test_point: Local<Option<piet::HitTestPoint>>,
    params: piet::PietParams,
) {
    let mut redraw = resized.iter().next().is_some();
    let window = params.text_params.windows.primary();
    let width = window.width() as f64;
    let height = window.height() as f64;
    let window_rect = kurbo::Rect::default().with_size((width, height));
    let center = window_rect.center();

    if let Some(layout) = &*layout {
        if let Some(event) = cursor_moved.iter().last() {
            let rect = kurbo::Rect::from_center_size(center, layout.size());
            // cursor position relative to the layout
            let cursor =
                (kurbo::Vec2::new(event.position.x as f64, height - event.position.y as f64)
                    - rect.origin().to_vec2())
                .to_point();

            let rect = rect.with_origin(kurbo::Point::ZERO);
            let h = if rect.contains(cursor) {
                Some(layout.hit_test_point(cursor))
            } else {
                None
            };
            if h != *hit_test_point {
                *hit_test_point = h;
                redraw = true;
            }
        }
    }

    if layout.is_none() || redraw {
        let mut piet = Piet::new(params);
        let family = FontFamily::new_unchecked("Vollkorn-Regular.ttf");
        *layout = if let Ok(layout) = piet
            .text()
            .new_text_layout("Hello,\npiet. ")
            .font(family, 64.0)
            .build()
        {
            piet.clear(None, piet::Color::TRANSPARENT);

            let rect = kurbo::Rect::from_center_size(center, layout.size());

            piet.draw_image(
                &*image,
                kurbo::Rect::from_center_size(center, (256.0, 256.0)),
                piet::InterpolationMode::Bilinear,
            );

            piet.fill(rect, &piet::Color::WHITE);

            let image_bounds = layout.image_bounds() + rect.origin().to_vec2();
            piet.fill(&image_bounds, &piet::Color::AQUA.with_alpha(0.4));

            let bg = piet::Color::GRAY.with_alpha(0.4);
            let yellow = piet::Color::YELLOW.with_alpha(0.4);
            for glyph in layout.glyphs.iter() {
                let glyph_rect = layout.glyph_rect(glyph) + rect.origin().to_vec2();
                piet.fill(glyph_rect, {
                    match &*hit_test_point {
                        Some(h) => {
                            if h.idx == glyph.byte_index {
                                if h.is_inside {
                                    &piet::Color::YELLOW
                                } else {
                                    &yellow
                                }
                            } else {
                                &bg
                            }
                        }
                        _ => &bg,
                    }
                });
            }

            piet.draw_text(&layout, rect.origin());
            Some(layout)
        } else {
            // font is still loading
            None
        }
    }
}
