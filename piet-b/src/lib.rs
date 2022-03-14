use bevy::{ecs::system::SystemParam, prelude::*, ui::UiImage};
pub use piet::kurbo;
use piet::RenderContext;

#[derive(SystemParam)]
pub struct PietParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: ResMut<'w, AssetServer>,
    pub nodes: Query<
        'w,
        's,
        (
            &'static Node,
            &'static UiColor,
            &'static UiImage,
            &'static Transform,
        ),
    >,
}

pub struct Piet<'w, 's> {
    params: PietParams<'w, 's>,
}

impl<'w, 's> Piet<'w, 's> {
    pub fn new(params: PietParams<'w, 's>) -> Self {
        Self { params }
    }
}

impl<'w, 's> RenderContext for Piet<'w, 's> {
    type Brush = Brush;
    type Text = PietText;
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
        gradient: impl Into<piet::FixedGradient>,
    ) -> Result<Self::Brush, piet::Error> {
        todo!()
    }

    fn clear(&mut self, region: impl Into<Option<kurbo::Rect>>, color: piet::Color) {
        todo!()
    }

    fn stroke(&mut self, shape: impl kurbo::Shape, brush: &impl piet::IntoBrush<Self>, width: f64) {
        todo!()
    }

    fn stroke_styled(
        &mut self,
        shape: impl kurbo::Shape,
        brush: &impl piet::IntoBrush<Self>,
        width: f64,
        style: &piet::StrokeStyle,
    ) {
        todo!()
    }

    fn fill(&mut self, shape: impl kurbo::Shape, brush: &impl piet::IntoBrush<Self>) {
        if let Some(rect) = shape.as_rect() {
            let brush = brush.make_brush(self, || shape.bounding_box()).into_owned();
            let Brush::Solid(color) = brush;
            let (r, g, b, a) = color.as_rgba8();
            let color = Color::rgba_u8(r, g, b, a);
            let size = rect.size();
            self.params.commands.spawn_bundle(NodeBundle {
                node: Node {
                    size: Vec2::new(size.width as f32, size.height as f32),
                },
                color: UiColor(color),
                ..Default::default()
            });
        }
    }

    fn fill_even_odd(&mut self, shape: impl kurbo::Shape, brush: &impl piet::IntoBrush<Self>) {
        todo!()
    }

    fn clip(&mut self, shape: impl kurbo::Shape) {
        todo!()
    }

    fn text(&mut self) -> &mut Self::Text {
        todo!()
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

    fn transform(&mut self, transform: kurbo::Affine) {
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
        image: &Self::Image,
        src_rect: impl Into<kurbo::Rect>,
        dst_rect: impl Into<kurbo::Rect>,
        interp: piet::InterpolationMode,
    ) {
        todo!()
    }

    fn capture_image_area(
        &mut self,
        src_rect: impl Into<kurbo::Rect>,
    ) -> Result<Self::Image, piet::Error> {
        todo!()
    }

    fn blurred_rect(
        &mut self,
        rect: kurbo::Rect,
        blur_radius: f64,
        brush: &impl piet::IntoBrush<Self>,
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
        piet: &mut Piet<'w, 's>,
        bbox: impl FnOnce() -> kurbo::Rect,
    ) -> std::borrow::Cow<'b, Brush> {
        std::borrow::Cow::Borrowed(self)
    }
}

#[derive(Clone)]
pub struct PietText;

impl piet::Text for PietText {
    type TextLayoutBuilder = PietTextLayoutBuilder;
    type TextLayout = PietTextLayout;

    fn font_family(&mut self, family_name: &str) -> Option<piet::FontFamily> {
        todo!()
    }

    fn load_font(&mut self, data: &[u8]) -> Result<piet::FontFamily, piet::Error> {
        todo!()
    }

    fn new_text_layout(&mut self, text: impl piet::TextStorage) -> Self::TextLayoutBuilder {
        todo!()
    }
}

#[derive(Clone)]
pub struct PietTextLayout;

impl piet::TextLayout for PietTextLayout {
    fn size(&self) -> kurbo::Size {
        todo!()
    }

    fn trailing_whitespace_width(&self) -> f64 {
        todo!()
    }

    fn image_bounds(&self) -> kurbo::Rect {
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

pub struct PietTextLayoutBuilder;

impl piet::TextLayoutBuilder for PietTextLayoutBuilder {
    type Out = PietTextLayout;

    fn max_width(self, width: f64) -> Self {
        todo!()
    }

    fn alignment(self, alignment: piet::TextAlignment) -> Self {
        todo!()
    }

    fn default_attribute(self, attribute: impl Into<piet::TextAttribute>) -> Self {
        todo!()
    }

    fn range_attribute(
        self,
        range: impl std::ops::RangeBounds<usize>,
        attribute: impl Into<piet::TextAttribute>,
    ) -> Self {
        todo!()
    }

    fn build(self) -> Result<Self::Out, piet::Error> {
        todo!()
    }
}

#[derive(Clone)]
pub struct PietImage;

impl piet::Image for PietImage {
    fn size(&self) -> kurbo::Size {
        todo!()
    }
}

pub fn piet_system(mut params: PietParams) {
    let piet = Piet::new(params);
}

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
        app.register_type::<Node>()
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
