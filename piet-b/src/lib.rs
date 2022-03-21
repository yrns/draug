use bevy::{
    ecs::system::SystemParam,
    math::{Size, Vec2},
    prelude::{
        App, AssetServer, Assets, Bundle, CameraUi, Commands, Entity, GlobalTransform, Image,
        Plugin, Query, Res, ResMut, TextureAtlas, Transform, Visibility,
    },
    render::camera::CameraTypePlugin,
    text::{
        DefaultTextPipeline, Font, FontAtlasSet, HorizontalAlign, PositionedGlyph, TextAlignment,
        TextError, TextSection, TextStyle, VerticalAlign,
    },
    ui::{Node, UiColor, UiImage},
    window::Windows,
};
use glyph_brush_layout::ab_glyph::{self, ScaleFont};
use piet::TextAttribute;
use std::{cell::RefCell, rc::Rc};

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
    pub textures: ResMut<'w, Assets<Image>>,
    pub fonts: Res<'w, Assets<Font>>,
    pub windows: Res<'w, Windows>,
    pub texture_atlases: ResMut<'w, Assets<TextureAtlas>>,
    pub font_atlas_set_storage: ResMut<'w, Assets<FontAtlasSet>>,
    pub text_pipeline: ResMut<'w, DefaultTextPipeline>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s usize>,
}

pub struct Piet<'w, 's> {
    commands: Rc<RefCell<Commands<'w, 's>>>,
    asset_server: Rc<Res<'w, AssetServer>>,
    nodes: NodesQuery<'w, 's>,
    text_nodes: TextNodesQuery<'w, 's>,
    text: PietText<'w, 's>,
}

impl<'w, 's> Piet<'w, 's> {
    pub fn new(params: PietParams<'w, 's>) -> Self {
        let PietParams {
            commands,
            asset_server,
            nodes,
            text_nodes,
            text_params,
        } = params;
        let commands = Rc::new(RefCell::new(commands));
        let asset_server = Rc::new(asset_server);
        let text = PietText::new(commands.clone(), asset_server.clone(), text_params);
        Self {
            commands,
            asset_server,
            nodes,
            text_nodes,
            text,
        }
    }

    pub fn window_rect(&self) -> kurbo::Rect {
        let window = self.text.windows.primary();
        let width = window.width() as f64;
        let height = window.height() as f64;
        kurbo::Rect::default().with_size((width, height))
    }
}

fn convert_color(color: piet::Color) -> bevy::prelude::Color {
    let (r, g, b, a) = color.as_rgba8();
    bevy::prelude::Color::rgba_u8(r, g, b, a)
}

fn convert_alignment(alignment: piet::TextAlignment) -> TextAlignment {
    match alignment {
        // ignoring right to left for now
        piet::TextAlignment::Start => TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Left,
        },
        piet::TextAlignment::End => TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Right,
        },
        piet::TextAlignment::Center => TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Center,
        },
        piet::TextAlignment::Justified => unimplemented!(),
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
        todo!()
    }

    fn clear(&mut self, region: impl Into<Option<kurbo::Rect>>, color: piet::Color) {
        match region.into() {
            Some(_) => unimplemented!(),
            None => {
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

    fn stroke(
        &mut self,
        _shape: impl kurbo::Shape,
        _brush: &impl piet::IntoBrush<Self>,
        _width: f64,
    ) {
        todo!()
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
        if let Some(rect) = shape.as_rect() {
            let brush = brush.make_brush(self, || shape.bounding_box()).into_owned();
            let Brush::Solid(color) = brush;
            let color = convert_color(color);
            let size = rect.size();
            let center = rect.center();
            self.commands.borrow_mut().spawn_bundle(NodeBundle {
                node: Node {
                    size: Vec2::new(size.width as f32, size.height as f32),
                },
                color: UiColor(color),
                transform: Transform::from_xyz(center.x as f32, center.y as f32, 0.0),
                ..Default::default()
            });
        } else {
            unimplemented!()
        }
    }

    fn fill_even_odd(&mut self, _shape: impl kurbo::Shape, _brush: &impl piet::IntoBrush<Self>) {
        todo!()
    }

    fn clip(&mut self, _shape: impl kurbo::Shape) {
        todo!()
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    fn draw_text(&mut self, layout: &Self::TextLayout, pos: impl Into<kurbo::Point>) {
        let rect = kurbo::Rect::from_origin_size(pos.into(), layout.size);
        let center = rect.center();
        // There is no way to tell if get_or_spawn fails (if the
        // entity is bad); we end up with a dangling transform?
        self.commands
            .borrow_mut()
            .get_or_spawn(layout.entity)
            .insert(Transform::from_xyz(center.x as f32, center.y as f32, 0.0))
            .insert(Node {
                size: Vec2::new(layout.size.width as f32, layout.size.height as f32),
            });
    }

    fn save(&mut self) -> Result<(), piet::Error> {
        todo!()
    }

    fn restore(&mut self) -> Result<(), piet::Error> {
        todo!()
    }

    fn finish(&mut self) -> Result<(), piet::Error> {
        todo!()
    }

    fn transform(&mut self, _transform: kurbo::Affine) {
        todo!()
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: piet::ImageFormat,
    ) -> Result<Self::Image, piet::Error> {
        todo!()
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<kurbo::Rect>,
        interp: piet::InterpolationMode,
    ) {
        todo!()
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
    pub commands: Rc<RefCell<Commands<'w, 's>>>,
    pub asset_server: Rc<Res<'w, AssetServer>>,
    pub textures: Rc<RefCell<ResMut<'w, Assets<Image>>>>,
    pub fonts: Rc<Res<'w, Assets<Font>>>,
    pub windows: Rc<Res<'w, Windows>>,
    pub texture_atlases: Rc<RefCell<ResMut<'w, Assets<TextureAtlas>>>>,
    pub font_atlas_set_storage: Rc<RefCell<ResMut<'w, Assets<FontAtlasSet>>>>,
    pub text_pipeline: Rc<RefCell<ResMut<'w, DefaultTextPipeline>>>,
}

impl<'w, 's> PietText<'w, 's> {
    pub fn new(
        commands: Rc<RefCell<Commands<'w, 's>>>,
        asset_server: Rc<Res<'w, AssetServer>>,
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
            textures: Rc::new(textures.into()),
            fonts: Rc::new(fonts),
            windows: Rc::new(windows),
            texture_atlases: Rc::new(texture_atlases.into()),
            font_atlas_set_storage: Rc::new(font_atlas_set_storage.into()),
            text_pipeline: Rc::new(text_pipeline.into()),
        }
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
            text: Rc::new(text),
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
#[derive(Clone)]
pub struct PietTextLayout {
    pub entity: Entity,
    pub text: Rc<dyn piet::TextStorage>,
    // we don't need these anymore after generating line metrics?
    pub glyphs: Rc<Vec<PositionedGlyph>>,
    pub size: kurbo::Size,
    pub line_metrics: Rc<[piet::LineMetric]>,
    //pub line_breaks: Rc<[]>,
}

pub fn glyph_rect(glyph: &PositionedGlyph) -> kurbo::Rect {
    // the glyph position is the center
    kurbo::Rect::from_center_size(
        (glyph.position.x as f64, glyph.position.y as f64),
        (glyph.size.x as f64, glyph.size.y as f64),
    )
}

impl PietTextLayout {
    // Returns a slice of glyphs in the specified byte range. This
    // assumes glyphs are sorted by byte_index.
    fn glyph_range(&self, range: std::ops::Range<usize>) -> Option<&[PositionedGlyph]> {
        self.glyphs
            .iter()
            .position(|g| g.byte_index >= range.start)
            .and_then(|start| {
                self.glyphs[start..]
                    .iter()
                    .rposition(|g| g.byte_index <= range.end)
                    .map(|end| &self.glyphs[start..end])
            })
    }
}

impl piet::TextLayout for PietTextLayout {
    fn size(&self) -> kurbo::Size {
        // this is CalculatedSize? it needs to include cursor height
        // for empty text
        self.size
    }

    fn trailing_whitespace_width(&self) -> f64 {
        todo!()
    }

    fn image_bounds(&self) -> kurbo::Rect {
        // position + size()?
        todo!()
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
            .and_then(|l| self.glyph_range(l.range()))
            .and_then(|gs| {
                if let Some(g) = gs.iter().find(|g| glyph_rect(g).contains(point)) {
                    Some(piet::HitTestPoint::new(g.byte_index, true))
                } else {
                    let point = Vec2::new(point.x as f32, point.y as f32);
                    // min_by_key requires Ord
                    gs.iter()
                        .map(|g| (g.position.distance_squared(point), g.byte_index))
                        .reduce(|a, b| if b.0 < a.0 { b } else { a })
                        .map(|(_, byte_index)| piet::HitTestPoint::new(byte_index, false))
                }
            })
            .unwrap_or_default()
    }

    fn hit_test_text_position(&self, idx: usize) -> piet::HitTestPosition {
        todo!()
    }
}

// TODO: split attributes into sections?
pub struct PietTextLayoutBuilder<'w, 's> {
    text: Rc<dyn piet::TextStorage>,
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

    fn default_attribute(mut self, attr: impl Into<TextAttribute>) -> Self {
        match attr.into() {
            TextAttribute::TextColor(color) => self.color = color,
            TextAttribute::FontFamily(font) => self.font = Some(font),
            TextAttribute::FontSize(size) => self.size = size,
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
        // TODO: via druid?
        let scale_factor = 1.0;

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

                let size = text_layout_info.size;
                let size = kurbo::Size::new(size.width as f64, size.height as f64);
                let glyphs = text_layout_info.glyphs.clone();

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
                        // This does not include trailing whitespace
                        // since we're building off glyphs. FIX:
                        end_offset: glyphs[end - 1].byte_index,
                        // TODO:
                        trailing_whitespace: 0,
                        baseline: *ascent,
                        height: *height,
                        y_offset,
                    });
                    y_offset += *height + *line_gap;
                }

                Ok(PietTextLayout {
                    entity,
                    text: self.text.clone(),
                    glyphs: Rc::new(glyphs),
                    size,
                    line_metrics: line_metrics.into(),
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
    if lines.len() > 0 {
        lines.push((start, glyphs.len()));
    }

    lines
}

#[derive(Clone)]
pub struct PietImage;

impl piet::Image for PietImage {
    fn size(&self) -> kurbo::Size {
        todo!()
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
