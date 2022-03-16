use std::{cell::RefCell, rc::Rc};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::camera::CameraTypePlugin,
    text::{DefaultTextPipeline, FontAtlasSet},
    ui::UiImage,
};
pub use piet::kurbo;
use piet::{RenderContext, TextAttribute};

pub type NodesQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        &'static UiColor,
        &'static UiImage,
        &'static Transform,
    ),
>;

#[derive(SystemParam)]
pub struct PietParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub nodes: NodesQuery<'w, 's>,
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
    text: PietText<'w, 's>,
}

impl<'w, 's> Piet<'w, 's> {
    pub fn new(params: PietParams<'w, 's>) -> Self {
        let PietParams {
            commands,
            asset_server,
            nodes,
            text_params,
        } = params;
        let commands = Rc::new(RefCell::new(commands));
        let asset_server = Rc::new(asset_server);
        let text = PietText::new(commands.clone(), asset_server.clone(), text_params);
        Self {
            commands,
            asset_server,
            nodes,
            text,
        }
    }
}

fn convert_color(color: piet::Color) -> Color {
    let (r, g, b, a) = color.as_rgba8();
    Color::rgba_u8(r, g, b, a)
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

impl<'w, 's> RenderContext for Piet<'w, 's> {
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

    fn clear(&mut self, _region: impl Into<Option<kurbo::Rect>>, _color: piet::Color) {
        todo!()
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
            self.commands.borrow_mut().spawn_bundle(NodeBundle {
                node: Node {
                    size: Vec2::new(size.width as f32, size.height as f32),
                },
                color: UiColor(color),
                ..Default::default()
            });
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
        todo!()
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
            text: text.as_str().to_string(),
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
pub struct PietTextLayout;

impl piet::TextLayout for PietTextLayout {
    fn size(&self) -> kurbo::Size {
        // this is CalculatedSize? it needs to include cursor height
        // for empty text
        todo!()
    }

    fn trailing_whitespace_width(&self) -> f64 {
        todo!()
    }

    fn image_bounds(&self) -> kurbo::Rect {
        // position + size()?
        todo!()
    }

    fn text(&self) -> &str {
        todo!()
    }

    fn line_text(&self, line_number: usize) -> Option<&str> {
        todo!()
    }

    fn line_metric(&self, line_number: usize) -> Option<piet::LineMetric> {
        todo!()
    }

    fn line_count(&self) -> usize {
        todo!()
    }

    fn hit_test_point(&self, point: kurbo::Point) -> piet::HitTestPoint {
        todo!()
    }

    fn hit_test_text_position(&self, idx: usize) -> piet::HitTestPosition {
        todo!()
    }
}

// TODO: split attributes into sections?
pub struct PietTextLayoutBuilder<'w, 's> {
    text: String,
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
        // TODO:
        let scale_factor = 1.0;

        let node_size = Size::new(self.max_width as f32, f32::MAX);

        let mut text_pipeline = self.params.text_pipeline.borrow_mut();
        let mut font_atlas_set_storage = self.params.font_atlas_set_storage.borrow_mut();
        let mut texture_atlases = self.params.texture_atlases.borrow_mut();
        let mut textures = self.params.textures.borrow_mut();

        // make one text section until we do attributes
        let alignment = convert_alignment(self.alignment);
        let text = Text {
            sections: vec![TextSection {
                value: self.text.clone(),
                style: TextStyle {
                    font: self.params.asset_server.load(self.font.unwrap().name()),
                    font_size: self.size as f32,
                    color: convert_color(self.color),
                },
            }],
            alignment,
        };

        let entity = self
            .params
            .commands
            .borrow_mut()
            .spawn_bundle(TextBundle {
                // fix extra clone?
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
            Err(e) => panic!("fatal error: {}", e),
            Ok(()) => {
                // TODO: store line metrics, entity
                let _text_layout_info = text_pipeline
                    .get_glyphs(&entity)
                    .expect("Failed to get glyphs from the pipeline that have just been computed");

                Ok(PietTextLayout {})
            }
        }
    }
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
    pub text: Text,
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
