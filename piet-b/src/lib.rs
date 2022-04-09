use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    math::{Affine2, Affine3A, Mat3A, Size, Vec2},
    prelude::{
        App, AssetServer, Assets, Bundle, CameraUi, Commands, Component, Entity, GlobalTransform,
        Handle, Image as BevyImage, Plugin, Query, Res, ResMut, TextureAtlas, Transform,
        Visibility,
    },
    render::{
        camera::CameraTypePlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    text::{
        DefaultTextPipeline, Font, FontAtlasSet, HorizontalAlign, PositionedGlyph, TextError,
        TextSection, TextStyle, VerticalAlign,
    },
    ui::{CalculatedClip, Node, UiColor, UiImage},
    window::{WindowId, Windows},
};
use glyph_brush_layout::ab_glyph::{self, ScaleFont};
use std::{cell::RefCell, sync::Arc};

// Piet is reexported; all collisions are prefixed/aliased.
pub use piet::kurbo;
pub use piet::*;

pub type NodesQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Node,
        &'static UiColor,
        &'static UiImage,
        &'static Transform,
    ),
>;

pub type TextNodesQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Node,
        &'static bevy::text::Text,
        &'static Transform,
    ),
>;

#[derive(SystemParam)]
pub struct PietParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub nodes: NodesQuery<'w, 's>,
    pub text_nodes: TextNodesQuery<'w, 's>,
    pub text_params: PietTextParams<'w, 's>,
}

#[derive(SystemParam)]
pub struct PietTextParams<'w, 's> {
    pub textures: ResMut<'w, Assets<BevyImage>>,
    pub fonts: Res<'w, Assets<Font>>,
    pub windows: Res<'w, Windows>,
    pub texture_atlases: ResMut<'w, Assets<TextureAtlas>>,
    pub font_atlas_set_storage: ResMut<'w, Assets<FontAtlasSet>>,
    pub text_pipeline: ResMut<'w, DefaultTextPipeline>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s usize>,
}

#[derive(Clone, Default)]
pub struct State {
    transform: Affine2,           //kurbo::Affine,
    clip: Option<CalculatedClip>, //Option<kurbo::Rect>,
}

pub struct Piet<'w, 's> {
    commands: Arc<RefCell<Commands<'w, 's>>>,
    nodes: NodesQuery<'w, 's>,
    text_nodes: TextNodesQuery<'w, 's>,
    text: PietText<'w, 's>,
    state: State,
    state_stack: Vec<State>,
    flip_y: Affine2,
}

impl<'w, 's> Piet<'w, 's> {
    /// `height` is the height of the drawable area in dp.
    pub fn new(params: PietParams<'w, 's>, height: f32) -> Self {
        let PietParams {
            commands,
            asset_server,
            nodes,
            text_nodes,
            text_params,
        } = params;
        let commands = Arc::new(RefCell::new(commands));
        let asset_server = Arc::new(asset_server);
        let text = PietText::new(commands.clone(), asset_server, text_params);

        let flip_y = Affine2::from_cols_array(&[1.0, 0., 0., -1.0, 0., height]);

        Self {
            commands,
            nodes,
            text_nodes,
            text,
            state: State::default(),
            state_stack: Vec::new(),
            flip_y,
        }
    }

    // Just save on the height on create.
    pub fn window_rect(&self) -> kurbo::Rect {
        let window = self.text.windows.primary();
        let width = window.width() as f64;
        let height = window.height() as f64;
        kurbo::Rect::default().with_size((width, height))
    }

    pub fn make_transform(&self, pt: kurbo::Point) -> Transform {
        let affine =
            Affine2::from_translation(Vec2::new(pt.x as f32, pt.y as f32)) * self.state.transform;

        // TODO:
        let z = 0.0;

        let aff3 = Affine3A {
            matrix3: Mat3A::from_mat2(affine.matrix2),
            translation: self
                .flip_y
                .transform_point2(affine.translation)
                .extend(z)
                .into(),
        };

        Transform::from_matrix(aff3.into())
    }
}

fn convert_color(color: piet::Color) -> bevy::prelude::Color {
    let (r, g, b, a) = color.as_rgba8();
    bevy::prelude::Color::rgba_u8(r, g, b, a)
}

fn convert_alignment(alignment: piet::TextAlignment) -> bevy::text::TextAlignment {
    match alignment {
        // ignoring right to left for now
        piet::TextAlignment::Start => bevy::text::TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Left,
        },
        piet::TextAlignment::End => bevy::text::TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Right,
        },
        piet::TextAlignment::Center => bevy::text::TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Center,
        },
        piet::TextAlignment::Justified => unimplemented!(),
    }
}

fn convert_image_format(format: piet::ImageFormat) -> TextureFormat {
    match format {
        ImageFormat::Grayscale => TextureFormat::R8Unorm,
        ImageFormat::Rgb => unimplemented!(),
        ImageFormat::RgbaSeparate => TextureFormat::Rgba32Uint,
        ImageFormat::RgbaPremul => TextureFormat::Rgba32Uint,
        _ => unimplemented!(),
    }
}

// Many of the default widgets use rounded rects, just map them to
// rects for now.
fn as_rect(shape: &impl kurbo::Shape) -> Option<kurbo::Rect> {
    shape
        .as_rect()
        .or_else(|| shape.as_rounded_rect().map(|r| r.rect()))
}

trait MaybeInsert {
    fn maybe_insert(&mut self, component: Option<impl Component>) -> &mut Self;
}

impl MaybeInsert for EntityCommands<'_, '_, '_> {
    fn maybe_insert(&mut self, component: Option<impl Component>) -> &mut Self {
        if let Some(component) = component {
            self.insert(component)
        } else {
            self
        }
    }
}

impl<'w, 's> piet::RenderContext for Piet<'w, 's> {
    type Brush = Brush;
    type Text = PietText<'w, 's>;
    type TextLayout = PietTextLayout;
    type Image = PietImage;

    fn status(&mut self) -> Result<(), piet::Error> {
        todo!()
    }

    fn solid_brush(&mut self, color: piet::Color) -> Self::Brush {
        Brush::Solid(color)
    }

    fn gradient(
        &mut self,
        _gradient: impl Into<piet::FixedGradient>,
    ) -> Result<Self::Brush, piet::Error> {
        // TODO: This is just so the default Druid widgets won't panic.
        Ok(Brush::Solid(piet::Color::GRAY))
    }

    // TODO: Partial clearing of entities.
    fn clear(&mut self, region: impl Into<Option<kurbo::Rect>>, color: piet::Color) {
        match region.into() {
            //Some(_) => unimplemented!(),
            _ => {
                {
                    let mut commands = self.commands.borrow_mut();
                    for (entity, ..) in self.nodes.iter() {
                        commands.entity(entity).despawn();
                    }
                    for (entity, ..) in self.text_nodes.iter() {
                        commands.entity(entity).despawn();
                    }
                }
                if color != piet::Color::TRANSPARENT {
                    self.fill(self.window_rect(), &color);
                }
            }
        }
    }

    fn stroke(&mut self, shape: impl kurbo::Shape, brush: &impl piet::IntoBrush<Self>, width: f64) {
        // TODO: This is just so the default Druid widgets won't panic.
        if let Some(rect) = as_rect(&shape) {
            // Outer stroke?
            self.fill(rect.inset(width), brush)
        }
    }

    fn stroke_styled(
        &mut self,
        _shape: impl kurbo::Shape,
        _brush: &impl piet::IntoBrush<Self>,
        _width: f64,
        _style: &piet::StrokeStyle,
    ) {
        todo!()
    }

    fn fill(&mut self, shape: impl kurbo::Shape, brush: &impl piet::IntoBrush<Self>) {
        if let Some(rect) = as_rect(&shape) {
            let brush = brush.make_brush(self, || shape.bounding_box()).into_owned();
            let Brush::Solid(color) = brush;
            let color = convert_color(color);
            let size = rect.size();

            let transform = self.make_transform(rect.center());

            self.commands
                .borrow_mut()
                .spawn_bundle(NodeBundle {
                    node: Node {
                        size: Vec2::new(size.width as f32, size.height as f32),
                    },
                    color: UiColor(color),
                    transform,
                    ..Default::default()
                })
                .maybe_insert(self.state.clip);
        } else {
            unimplemented!()
        }
    }

    fn fill_even_odd(&mut self, _shape: impl kurbo::Shape, _brush: &impl piet::IntoBrush<Self>) {
        todo!()
    }

    fn clip(&mut self, shape: impl kurbo::Shape) {
        if let Some(rect) = as_rect(&shape) {
            let kurbo::Rect { x0, y0, x1, y1 } = rect;
            let clip = CalculatedClip {
                clip: bevy::sprite::Rect {
                    min: Vec2::new(x0 as f32, y0 as f32),
                    max: Vec2::new(x1 as f32, y1 as f32),
                },
            };
            self.state.clip = Some(clip);
        }
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    // `pt` is the top-left of the layout.
    fn draw_text(&mut self, layout: &Self::TextLayout, pt: impl Into<kurbo::Point>) {
        let rect = kurbo::Rect::from_origin_size(pt.into(), layout.size);

        let transform = self.make_transform(rect.center());

        // There is no way to tell if get_or_spawn fails (if the
        // entity is bad); we end up with a dangling transform?
        self.commands
            .borrow_mut()
            .get_or_spawn(layout.entity)
            .insert(transform)
            .insert(Node {
                size: Vec2::new(layout.size.width as f32, layout.size.height as f32),
            })
            .maybe_insert(self.state.clip);
    }

    fn save(&mut self) -> Result<(), piet::Error> {
        // Retain current state.
        self.state_stack.push(self.state.clone());
        // self.state_stack.push(std::mem::take(&mut self.state));
        Ok(())
    }

    fn restore(&mut self) -> Result<(), piet::Error> {
        if let Some(state) = self.state_stack.pop() {
            self.state = state;
            Ok(())
        } else {
            Err(piet::Error::StackUnbalance)
        }
    }

    fn finish(&mut self) -> Result<(), piet::Error> {
        Ok(())
    }

    // *= ?
    fn transform(&mut self, transform: kurbo::Affine) {
        let coeffs = transform.as_coeffs().map(|a| a as f32);
        self.state.transform = self.state.transform * Affine2::from_cols_array(&coeffs);
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: piet::ImageFormat,
    ) -> Result<Self::Image, piet::Error> {
        let mut textures = self.text.textures.borrow_mut();
        let image = BevyImage::new_fill(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            buf,
            // ImageFormat::RgbaPremul needs to be handled in the
            // render pipeline
            convert_image_format(format),
        );
        let image = textures.add(image);
        Ok(image.into())
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<kurbo::Rect>,
        _interp: piet::InterpolationMode,
    ) {
        let rect = dst_rect.into();
        let size = rect.size();

        let transform = self.make_transform(rect.center());

        self.commands
            .borrow_mut()
            .spawn_bundle(NodeBundle {
                node: Node {
                    size: Vec2::new(size.width as f32, size.height as f32),
                },
                image: UiImage(image.0.clone()),
                transform,
                ..Default::default()
            })
            .maybe_insert(self.state.clip);
    }

    fn draw_image_area(
        &mut self,
        _image: &Self::Image,
        _src_rect: impl Into<kurbo::Rect>,
        _dst_rect: impl Into<kurbo::Rect>,
        _interp: piet::InterpolationMode,
    ) {
        todo!()
    }

    fn capture_image_area(
        &mut self,
        _src_rect: impl Into<kurbo::Rect>,
    ) -> Result<Self::Image, piet::Error> {
        todo!()
    }

    // generate an image via piet::utils?
    fn blurred_rect(
        &mut self,
        _rect: kurbo::Rect,
        _blur_radius: f64,
        _brush: &impl piet::IntoBrush<Self>,
    ) {
        todo!()
    }

    fn current_transform(&self) -> kurbo::Affine {
        todo!()
    }
}

#[derive(Clone)]
pub enum Brush {
    Solid(piet::Color),
}

impl<'w, 's> piet::IntoBrush<Piet<'w, 's>> for Brush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut Piet<'w, 's>,
        _bbox: impl FnOnce() -> kurbo::Rect,
    ) -> std::borrow::Cow<'b, Brush> {
        std::borrow::Cow::Borrowed(self)
    }
}

// This is ephemeral and is only valid for one frame. On finish check
// refs?
#[derive(Clone)]
pub struct PietText<'w, 's> {
    pub commands: Arc<RefCell<Commands<'w, 's>>>,
    pub asset_server: Arc<Res<'w, AssetServer>>,
    pub textures: Arc<RefCell<ResMut<'w, Assets<BevyImage>>>>,
    pub fonts: Arc<Res<'w, Assets<Font>>>,
    pub windows: Arc<Res<'w, Windows>>,
    pub texture_atlases: Arc<RefCell<ResMut<'w, Assets<TextureAtlas>>>>,
    pub font_atlas_set_storage: Arc<RefCell<ResMut<'w, Assets<FontAtlasSet>>>>,
    pub text_pipeline: Arc<RefCell<ResMut<'w, DefaultTextPipeline>>>,
}

impl<'w, 's> PietText<'w, 's> {
    pub fn new(
        commands: Arc<RefCell<Commands<'w, 's>>>,
        asset_server: Arc<Res<'w, AssetServer>>,
        params: PietTextParams<'w, 's>,
    ) -> Self {
        let PietTextParams {
            textures,
            fonts,
            windows,
            texture_atlases,
            font_atlas_set_storage,
            text_pipeline,
            ..
        } = params;

        Self {
            commands,
            asset_server,
            textures: Arc::new(textures.into()),
            fonts: fonts.into(),
            windows: windows.into(),
            texture_atlases: Arc::new(texture_atlases.into()),
            font_atlas_set_storage: Arc::new(font_atlas_set_storage.into()),
            text_pipeline: Arc::new(text_pipeline.into()),
        }
    }
}

impl<'w, 's> PietText<'w, 's> {
    pub fn scale_factor(&self) -> f64 {
        // TODO: Multiple windows.
        self.windows.scale_factor(WindowId::primary())
    }
}

impl<'w, 's> piet::Text for PietText<'w, 's> {
    type TextLayoutBuilder = PietTextLayoutBuilder<'w, 's>;
    type TextLayout = PietTextLayout;

    fn font_family(&mut self, family_name: &str) -> Option<piet::FontFamily> {
        Some(piet::FontFamily::new_unchecked(family_name))
    }

    fn load_font(&mut self, _data: &[u8]) -> Result<piet::FontFamily, piet::Error> {
        unimplemented!()
    }

    fn new_text_layout(&mut self, text: impl piet::TextStorage) -> Self::TextLayoutBuilder {
        Self::TextLayoutBuilder {
            // Layouts need to fulfill Resource so clone now to Arc<str>.
            text: text.as_str().to_string().into(),
            params: self.clone(),
            max_width: f64::MAX,
            alignment: piet::TextAlignment::Start,
            color: piet::util::DEFAULT_TEXT_COLOR,
            font: None,
            size: piet::util::DEFAULT_FONT_SIZE,
        }
    }
}

// This struct persists inside widgets and needs to hold onto
// everything it needs to fulfill the impl.
#[derive(Clone, Debug)]
pub struct PietTextLayout {
    pub entity: Entity,
    pub text: Arc<str>,
    // we don't need these anymore after generating line metrics?
    pub glyphs: Arc<Vec<PositionedGlyph>>,
    pub size: kurbo::Size,
    pub line_metrics: Arc<[piet::LineMetric]>,
    pub image_bounds: kurbo::Rect,
}

impl PietTextLayout {
    pub fn glyph_rect(&self, glyph: &PositionedGlyph) -> kurbo::Rect {
        // the glyph position is the center
        kurbo::Rect::from_center_size(
            (
                glyph.position.x as f64,
                // Bevy is y-up, Piet is y-down.
                self.size.height - glyph.position.y as f64,
            ),
            (glyph.size.x as f64, glyph.size.y as f64),
        )
    }

    // Returns a slice of glyphs in the specified byte range. This
    // assumes glyphs are sorted by byte_index.
    fn glyph_range(&self, range: std::ops::Range<usize>) -> Option<&[PositionedGlyph]> {
        self.glyphs
            .iter()
            .position(|g| g.byte_index >= range.start)
            .and_then(|start| {
                self.glyphs[start..]
                    .iter()
                    .rposition(|g| g.byte_index < range.end)
                    .map(|end| {
                        let end = start + end + 1; // exclusive
                        &self.glyphs[start..end]
                    })
            })
    }
}

impl std::fmt::Display for PietTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chars: Vec<_> = self.text.chars().collect();
        let glyphs: Vec<_> = self
            .glyphs
            .iter()
            .map(|g| (g.byte_index, chars[g.byte_index]))
            .collect();
        write!(f, "{:?}", glyphs)
    }
}

impl piet::TextLayout for PietTextLayout {
    fn size(&self) -> kurbo::Size {
        // this is CalculatedSize? it needs to include cursor height
        // for empty text
        self.size
    }

    // TODO:
    fn trailing_whitespace_width(&self) -> f64 {
        0.
    }

    fn image_bounds(&self) -> kurbo::Rect {
        self.image_bounds
    }

    fn text(&self) -> &str {
        self.text.as_str()
    }

    fn line_text(&self, line_number: usize) -> Option<&str> {
        self.line_metrics
            .get(line_number)
            .map(|m| &self.text[m.range()])
    }

    fn line_metric(&self, line_number: usize) -> Option<piet::LineMetric> {
        // TODO: if line_number == 0 && self.text.is_empty() {
        self.line_metrics.get(line_number).cloned()
    }

    fn line_count(&self) -> usize {
        self.line_metrics.len()
    }

    fn hit_test_point(&self, point: kurbo::Point) -> piet::HitTestPoint {
        self.line_metrics
            .iter()
            .find(|l| point.y <= (l.y_offset + l.height))
            .or_else(|| self.line_metrics.last())
            // Save glyph ranges per line on build?
            .and_then(|l| self.glyph_range(l.range()))
            .and_then(|gs| {
                if let Some(g) = gs.iter().find(|g| self.glyph_rect(g).contains(point)) {
                    Some(piet::HitTestPoint::new(g.byte_index, true))
                } else {
                    // Find closest glyph to point via center. This
                    // should use the left edge.
                    let point = Vec2::new(point.x as f32, (self.size.height - point.y) as f32);
                    // min_by_key requires Ord
                    gs.iter()
                        .map(|g| (g.position.distance_squared(point), g.byte_index))
                        .reduce(|a, b| if b.0 < a.0 { b } else { a })
                        .map(|(_, byte_index)| piet::HitTestPoint::new(byte_index, false))
                }
            })
            .unwrap_or_default()
    }

    // If the offset is whitespace we will get the previous glyph?
    fn hit_test_text_position(&self, idx: usize) -> piet::HitTestPosition {
        let idx = idx.min(self.text.len());
        assert!(self.text.is_char_boundary(idx));
        let n = piet::util::line_number_for_position(&self.line_metrics, idx);
        let l = &self.line_metrics[n];
        let gs = self.glyph_range(l.range()).unwrap();
        let g = &gs[gs
            .binary_search_by_key(&idx, |g| g.byte_index)
            .unwrap_or_else(|n| n.saturating_sub(1))];
        let point = kurbo::Point::new(
            (g.position.x - g.size.x * 0.5) as f64,
            (l.y_offset + l.baseline) as f64,
        );

        piet::HitTestPosition::new(point, n)
    }
}

// TODO: split attributes into sections?
pub struct PietTextLayoutBuilder<'w, 's> {
    text: Arc<str>,
    params: PietText<'w, 's>,
    max_width: f64,
    alignment: piet::TextAlignment,
    color: piet::Color,
    font: Option<piet::FontFamily>,
    size: f64,
}

impl piet::TextLayoutBuilder for PietTextLayoutBuilder<'_, '_> {
    type Out = PietTextLayout;

    fn max_width(mut self, width: f64) -> Self {
        self.max_width = width;
        self
    }

    fn alignment(mut self, alignment: piet::TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn default_attribute(mut self, attr: impl Into<piet::TextAttribute>) -> Self {
        match attr.into() {
            piet::TextAttribute::TextColor(color) => self.color = color,
            piet::TextAttribute::FontFamily(font) => self.font = Some(font),
            piet::TextAttribute::FontSize(size) => self.size = size,
            _ => (),
        }
        self
    }

    fn range_attribute(
        self,
        _range: impl std::ops::RangeBounds<usize>,
        _attribute: impl Into<piet::TextAttribute>,
    ) -> Self {
        todo!()
    }

    // From text_system:
    fn build(self) -> Result<Self::Out, piet::Error> {
        let scale_factor = self.params.scale_factor();

        let node_size = Size::new(self.max_width as f32, f32::MAX);

        let mut text_pipeline = self.params.text_pipeline.borrow_mut();
        let mut font_atlas_set_storage = self.params.font_atlas_set_storage.borrow_mut();
        let mut texture_atlases = self.params.texture_atlases.borrow_mut();
        let mut textures = self.params.textures.borrow_mut();

        // make one text section until we do attributes
        let alignment = convert_alignment(self.alignment);
        let text = bevy::text::Text {
            sections: vec![TextSection {
                value: self.text.as_str().to_string(),
                style: TextStyle {
                    font: self
                        .params
                        .asset_server
                        .load(self.font.expect("missing font name").name()),
                    font_size: self.size as f32,
                    color: convert_color(self.color),
                },
            }],
            alignment,
        };

        // spawn and fail? FIX:
        let entity = self
            .params
            .commands
            .borrow_mut()
            .spawn_bundle(TextBundle {
                // fix extra clone?
                // we only need the section color for rendering
                text: text.clone(),
                ..Default::default()
            })
            .id();

        match text_pipeline.queue_text(
            entity,
            &self.params.fonts,
            &text.sections,
            scale_factor,
            alignment,
            node_size,
            &mut font_atlas_set_storage,
            &mut texture_atlases,
            &mut textures,
        ) {
            Ok(()) => {
                let text_layout_info = text_pipeline
                    .get_glyphs(&entity)
                    .expect("Failed to get glyphs from the pipeline that have just been computed");

                // Font metrics will be in pixel values, but we want
                // dp for Piet.
                let inv_scale = (1.0 / scale_factor) as f32;
                let size = text_layout_info.size * inv_scale;
                let size = kurbo::Size::new(size.width as f64, size.height as f64);
                let glyphs: Vec<_> = text_layout_info
                    .glyphs
                    .iter()
                    .cloned()
                    .map(|mut g| {
                        g.position *= inv_scale;
                        g.size *= inv_scale;
                        g
                    })
                    .collect();

                // Unfortunately we don't have access to line metrics
                // internal to glyph_brush_layout, so we have to
                // recreate them here. They may be inaccurate. In
                // addition, the lines heuristic may be inaccurate
                // with respect to the internal line breaker.
                let section_fonts = &text
                    .sections
                    .iter()
                    .map(|s| {
                        let TextStyle {
                            font, font_size, ..
                        } = &s.style;
                        // fonts are already checked above
                        (self.params.fonts.get(font.id).unwrap(), font_size)
                    })
                    //.unique() - probably dupes, but font_size isn't
                    // hashable; we need to maintain section indices
                    // as well
                    .map(|(font, size)| ab_glyph::Font::as_scaled(&font.font, *size))
                    .collect::<Vec<_>>();

                let mut y_offset = 0.0;
                let mut line_metrics = Vec::new();
                for (start, end) in lines(&glyphs).into_iter() {
                    let (ascent, _descent, height, line_gap) = &glyphs[start..end]
                        .iter()
                        .map(|g| {
                            let f = section_fonts[g.section_index];
                            (
                                f.ascent() as f64,
                                f.descent() as f64,
                                f.height() as f64,
                                f.line_gap() as f64,
                            )
                        })
                        .reduce(|m1, m2| {
                            (
                                m1.0.max(m2.0),
                                m1.1.max(m2.1),
                                m1.2.max(m2.2),
                                m1.3.max(m2.3),
                            )
                        })
                        // FIX: a line could be empty of glyphs (only
                        // whitespace) where this will fail
                        .unwrap();
                    // see: https://docs.rs/piet/latest/piet/struct.LineMetric.html
                    line_metrics.push(piet::LineMetric {
                        // These offsets only work because we only
                        // have one section. FIX:
                        start_offset: glyphs[start].byte_index,
                        // 1) This does not include trailing
                        // whitespace since we're building off
                        // glyphs. 2) This may be wrong w/ respect to
                        // utf8. FIX:
                        end_offset: glyphs[end - 1].byte_index + 1,
                        // TODO:
                        trailing_whitespace: 0,
                        baseline: *ascent,
                        height: *height,
                        y_offset,
                    });
                    y_offset += *height + *line_gap;
                }

                let image_bounds = glyphs
                    .iter()
                    .map(|g| {
                        kurbo::Rect::from_center_size(
                            (g.position.x as f64, size.height - g.position.y as f64),
                            (g.size.x as f64, g.size.y as f64),
                        )
                    })
                    .reduce(|r, gr| r.union(gr))
                    .unwrap_or_default();

                Ok(PietTextLayout {
                    entity,
                    text: self.text.clone(),
                    glyphs: glyphs.into(),
                    size,
                    line_metrics: line_metrics.into(),
                    image_bounds,
                })
            }
            Err(TextError::NoSuchFont) => {
                // font asset not loaded yet - what if the asset fails
                // to load?
                Err(piet::Error::MissingFont)
            }
            Err(e) => Err(piet::Error::BackendError(e.into())),
        }
    }
}

// Start a new line for any glyph where the glyph's x position is less
// than the previous glyph's.
fn lines(glyphs: &Vec<PositionedGlyph>) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_x = -f32::INFINITY;

    for (i, glyph) in glyphs.iter().enumerate() {
        let x = glyph.position.x;
        if x < last_x {
            // new line
            lines.push((start, i));
            start = i;
        }
        last_x = x;
    }

    // last line
    if glyphs.len() > 0 {
        lines.push((start, glyphs.len()));
    }

    lines
}

// Write a system to cache size here? Trigger relayout when an image
// loads if size() initially returns empty?
#[derive(Clone, Debug)]
pub struct PietImage(pub Handle<BevyImage>);

impl piet::Image for PietImage {
    fn size(&self) -> kurbo::Size {
        // needs resources, who calls this?
        todo!()
    }
}

impl From<Handle<BevyImage>> for PietImage {
    fn from(handle: Handle<BevyImage>) -> Self {
        Self(handle)
    }
}

//pub fn piet_system(mut params: PietParams) {
//let piet = Piet::new(params, text_params);
//}

#[derive(Bundle, Clone, Debug, Default)]
pub struct NodeBundle {
    pub node: Node,
    pub color: UiColor,
    pub image: UiImage,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
}

// is this needed? what about ImageMode and CalculatedSize?
pub struct ImageBundle {}

#[derive(Bundle, Clone, Debug, Default)]
pub struct TextBundle {
    pub node: Node,
    pub text: bevy::text::Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
}

#[derive(Default)]
pub struct PietPlugin;

impl Plugin for PietPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CameraTypePlugin::<CameraUi>::default())
            .register_type::<Node>()
            .register_type::<UiColor>()
            .register_type::<UiImage>();
        // render systems
        bevy::ui::build_ui_render(app);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
