use bevy::{prelude::*, window::WindowResized, winit::WinitSettings};

use piet_b::{
    self as piet, kurbo, FontFamily, Piet, PietImage, PietTextLayout, RenderContext, Text,
    TextLayout, TextLayoutBuilder,
};

fn main() {
    App::new()
        // This needs to be before WindowPlugin.
        // .insert_resource(bevy::window::WindowDescriptor {
        //     scale_factor_override: Some(1.0),
        //     ..Default::default()
        // })
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::ui::UiPlugin>()
        })
        // This replaces UiPlugin; can't have both.
        .add_plugin(piet::PietPlugin::default())
        // #5384
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(draw)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
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
            let text_rect = kurbo::Rect::from_center_size(center, layout.size());
            // cursor position relative to the layout
            let cursor =
                (kurbo::Vec2::new(event.position.x as f64, height - event.position.y as f64)
                    - text_rect.origin().to_vec2())
                .to_point();

            let text_rect = text_rect.with_origin(kurbo::Point::ZERO);
            let h = if text_rect.contains(cursor) {
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
        let scale_factor = window.scale_factor();
        let mut piet = Piet::new(params, height as f32);
        let family = FontFamily::new_unchecked("Vollkorn-Regular.ttf");
        *layout = if let Ok(layout) = piet
            .text()
            .new_text_layout("Hello,\npiet. ")
            .font(family.clone(), 64.0)
            .build()
        {
            piet.clear(None, piet::Color::TRANSPARENT);

            let text_rect = kurbo::Rect::from_center_size(center, layout.size());

            // The image looks bad scaled so we unscale it here. We
            // can't get the size of the image until it's loaded -
            // redraw on a load event?
            let image_rect = kurbo::Rect::from_center_size(
                center,
                kurbo::Size::new(256.0, 256.0) / scale_factor,
            );
            piet.draw_image(&*image, image_rect, piet::InterpolationMode::Bilinear);

            piet.fill(text_rect, &piet::Color::WHITE);

            let image_bounds = layout.image_bounds() + text_rect.origin().to_vec2();
            piet.fill(&image_bounds, &piet::Color::AQUA.with_alpha(0.4));

            let bg = piet::Color::GRAY.with_alpha(0.4);
            let yellow = piet::Color::YELLOW.with_alpha(0.4);
            for glyph in layout.glyphs.iter() {
                let glyph_rect = layout.glyph_rect(glyph) + text_rect.origin().to_vec2();
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

            piet.draw_text(&layout, text_rect.origin());

            // Clipped fill and text.
            piet.with_save(|ctx| {
                ctx.clip(image_rect);

                let layout2 = ctx
                    .text()
                    .new_text_layout("This text is clipped.")
                    .font(family, 32.0)
                    .build()
                    .unwrap();

                let size = layout2.size();

                // Lower left corner. Clipped near the baseline.
                let pt = kurbo::Point::new(image_rect.x0, image_rect.y1 - size.height * 0.65);
                ctx.draw_text(&layout2, pt);

                let bg = kurbo::Rect::from_origin_size(pt, size); //.inset(10.0);
                let purple = piet::Color::PURPLE.with_alpha(0.4);
                ctx.fill(bg, &purple);

                Ok(())
            })
            .unwrap();

            Some(layout)
        } else {
            // font is still loading
            None
        }
    }
}
