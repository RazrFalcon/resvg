use std::ffi::CString;
use std::i32;
use std::path::PathBuf;
use std::ops::{Deref, DerefMut};
use std::slice;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod ffi;

pub use ffi::qtc_qpainter;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FillRule {
    OddEven = ffi::FillRule_OddEvenFill as isize,
    Winding = ffi::FillRule_WindingFill as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LineCap {
    Flat = ffi::PenCapStyle_FlatCap as isize,
    Square = ffi::PenCapStyle_SquareCap as isize,
    Round = ffi::PenCapStyle_RoundCap as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Bevel = ffi::PenJoinStyle_BevelJoin as isize,
    Round = ffi::PenJoinStyle_RoundJoin as isize,
    Miter = ffi::PenJoinStyle_MiterJoin as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal = ffi::FontStyle_StyleNormal as isize,
    Italic = ffi::FontStyle_StyleItalic as isize,
    Oblique = ffi::FontStyle_StyleOblique as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Thin = ffi::FontWeight_Thin as isize,
    ExtraLight = ffi::FontWeight_ExtraLight as isize,
    Light = ffi::FontWeight_Light as isize,
    Normal = ffi::FontWeight_Normal as isize,
    Medium = ffi::FontWeight_Medium as isize,
    DemiBold = ffi::FontWeight_DemiBold as isize,
    Bold = ffi::FontWeight_Bold as isize,
    ExtraBold = ffi::FontWeight_ExtraBold as isize,
    Black = ffi::FontWeight_Black as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FontStretch {
    UltraCondensed = ffi::FontStretch_UltraCondensed as isize,
    ExtraCondensed = ffi::FontStretch_ExtraCondensed as isize,
    Condensed = ffi::FontStretch_Condensed as isize,
    SemiCondensed = ffi::FontStretch_SemiCondensed as isize,
    Unstretched = ffi::FontStretch_Unstretched as isize,
    SemiExpanded = ffi::FontStretch_SemiExpanded as isize,
    Expanded = ffi::FontStretch_Expanded as isize,
    ExtraExpanded = ffi::FontStretch_ExtraExpanded as isize,
    UltraExpanded = ffi::FontStretch_UltraExpanded as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompositionMode {
    SourceOver = ffi::CompositionMode_CompositionMode_SourceOver as isize,
    DestinationOver = ffi::CompositionMode_CompositionMode_DestinationOver as isize,
    Clear = ffi::CompositionMode_CompositionMode_Clear as isize,
    Source = ffi::CompositionMode_CompositionMode_Source as isize,
    Destination = ffi::CompositionMode_CompositionMode_Destination as isize,
    SourceIn = ffi::CompositionMode_CompositionMode_SourceIn as isize,
    DestinationIn = ffi::CompositionMode_CompositionMode_DestinationIn as isize,
    SourceOut = ffi::CompositionMode_CompositionMode_SourceOut as isize,
    DestinationOut = ffi::CompositionMode_CompositionMode_DestinationOut as isize,
    SourceAtop = ffi::CompositionMode_CompositionMode_SourceAtop as isize,
    DestinationAtop = ffi::CompositionMode_CompositionMode_DestinationAtop as isize,
    Xor = ffi::CompositionMode_CompositionMode_Xor as isize,
    Plus = ffi::CompositionMode_CompositionMode_Plus as isize,
    Multiply = ffi::CompositionMode_CompositionMode_Multiply as isize,
    Screen = ffi::CompositionMode_CompositionMode_Screen as isize,
    Overlay = ffi::CompositionMode_CompositionMode_Overlay as isize,
    Darken = ffi::CompositionMode_CompositionMode_Darken as isize,
    Lighten = ffi::CompositionMode_CompositionMode_Lighten as isize,
    ColorDodge = ffi::CompositionMode_CompositionMode_ColorDodge as isize,
    ColorBurn = ffi::CompositionMode_CompositionMode_ColorBurn as isize,
    HardLight = ffi::CompositionMode_CompositionMode_HardLight as isize,
    SoftLight = ffi::CompositionMode_CompositionMode_SoftLight as isize,
    Difference = ffi::CompositionMode_CompositionMode_Difference as isize,
    Exclusion = ffi::CompositionMode_CompositionMode_Exclusion as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AspectRatioMode {
    Ignore = ffi::AspectRatioMode_IgnoreAspectRatio as isize,
    Keep = ffi::AspectRatioMode_KeepAspectRatio as isize,
    KeepByExpanding = ffi::AspectRatioMode_KeepAspectRatioByExpanding as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PathSegmentType {
    MoveTo,
    LineTo,
    CurveTo,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Spread {
    Pad = ffi::Spread_PadSpread as isize,
    Reflect = ffi::Spread_ReflectSpread as isize,
    Repeat = ffi::Spread_RepeatSpread as isize,
}

pub struct GuiApp(*mut ffi::qtc_qguiapp);

impl GuiApp {
    pub fn new(app_name: &str) -> GuiApp {
        let c_app_name = CString::new(app_name).unwrap();
        unsafe { GuiApp(ffi::qtc_create_gui(c_app_name.as_ptr() as *mut _)) }
    }
}

impl Drop for GuiApp {
    fn drop(&mut self) {
        unsafe { ffi::qtc_destroy_gui(self.0) }
    }
}


pub struct Image(*mut ffi::qtc_qimage);

impl Image {
    pub fn new_rgba(width: u32, height: u32) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_create_rgba(width, height)) }
    }

    pub fn new_rgba_premultiplied(width: u32, height: u32) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_create_rgba_premultiplied(width, height)) }
    }

    pub fn from_file(path: &PathBuf) -> Option<Image> {
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        unsafe { Self::from_ptr(ffi::qtc_qimage_from_file(c_path.as_ptr())) }
    }

    pub fn from_data(data: &[u8]) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_from_data(data.as_ptr(), data.len() as i32)) }
    }

    unsafe fn from_ptr(img: *mut ffi::qtc_qimage) -> Option<Image> {
        if img.is_null() {
            None
        } else {
            Some(Image(img))
        }
    }

    pub fn fill(&mut self, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::qtc_qimage_fill(self.0, r, g, b, a) }
    }

    pub fn set_dpi(&mut self, dpi: f64) {
        unsafe { ffi::qtc_qimage_set_dpi(self.0, dpi) }
    }

    pub fn to_rgba(&self) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_to_rgba(self.0)) }
    }

    pub fn save(&self, path: &str) -> bool {
        let c_path = CString::new(path).unwrap();
        unsafe { ffi::qtc_qimage_save(self.0, c_path.as_ptr()) }
    }

    pub fn resize(&self, width: u32, height: u32, ratio: AspectRatioMode) -> Option<Image> {
        unsafe {
            Self::from_ptr(ffi::qtc_qimage_resize(self.0, width, height,
                                                  ratio as ffi::AspectRatioMode))
        }
    }

    pub fn copy(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_copy(self.0, x, y, width, height)) }
    }

    pub fn try_clone(&self) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_copy(self.0, 0, 0, self.width(), self.height())) }
    }

    pub fn data(&self) -> ImageData {
        unsafe {
            let ptr = ffi::qtc_qimage_get_data(self.0);
            let len = ffi::qtc_qimage_get_byte_count(self.0) as usize;

            ImageData {
                slice: slice::from_raw_parts_mut(ptr, len),
            }
        }
    }

    pub fn data_mut(&mut self) -> ImageData {
        unsafe {
            let ptr = ffi::qtc_qimage_get_data(self.0);
            let len = ffi::qtc_qimage_get_byte_count(self.0) as usize;

            ImageData {
                slice: slice::from_raw_parts_mut(ptr, len),
            }
        }
    }

    pub fn width(&self) -> u32 {
        unsafe { ffi::qtc_qimage_get_width(self.0) }
    }

    pub fn height(&self) -> u32 {
        unsafe { ffi::qtc_qimage_get_height(self.0) }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qimage_destroy(self.0) }
    }
}


pub struct ImageData<'a> {
    slice: &'a mut [u8],
}

impl<'a> Deref for ImageData<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.slice
    }
}

impl<'a> DerefMut for ImageData<'a> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.slice
    }
}


pub struct Painter(*mut ffi::qtc_qpainter, bool);

impl Painter {
    pub fn new(img: &mut Image) -> Painter {
        unsafe { Painter(ffi::qtc_qpainter_create(img.0), true) }
    }

    pub unsafe fn from_raw(ptr: *mut ffi::qtc_qpainter) -> Painter {
        Painter(ptr, false)
    }

    pub fn font(&self) -> Font {
        unsafe { Font(ffi::qtc_qpainter_get_font(self.0)) }
    }

    pub fn set_font(&mut self, font: &Font) {
        unsafe { ffi::qtc_qpainter_set_font(self.0, font.0) }
    }

    pub fn font_metrics(&self) -> FontMetricsF {
        unsafe { FontMetricsF(ffi::qtc_qpainter_get_fontmetricsf(self.0)) }
    }

    pub fn set_pen(&mut self, pen: Pen) {
        unsafe { ffi::qtc_qpainter_set_pen(self.0, pen.0) }
    }

    pub fn reset_pen(&mut self) {
        unsafe { ffi::qtc_qpainter_reset_pen(self.0) }
    }

    pub fn set_brush(&mut self, brush: Brush) {
        unsafe { ffi::qtc_qpainter_set_brush(self.0, brush.0) }
    }

    pub fn reset_brush(&mut self) {
        unsafe { ffi::qtc_qpainter_reset_brush(self.0) }
    }

    pub fn set_opacity(&mut self, opacity: f64) {
        unsafe { ffi::qtc_qpainter_set_opacity(self.0, opacity) }
    }

    pub fn draw_path(&mut self, path: &PainterPath) {
        unsafe { ffi::qtc_qpainter_draw_path(self.0, path.0) }
    }

    pub fn draw_image(&mut self, x: f64, y: f64, img: &Image) {
        unsafe { ffi::qtc_qpainter_draw_image(self.0, x, y, img.0) }
    }

    pub fn draw_text(&mut self, x: f64, y: f64, text: &str) {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::qtc_qpainter_draw_text(self.0, x, y, c_text.as_ptr()) }
    }

    pub fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        unsafe { ffi::qtc_qpainter_draw_rect(self.0, x, y, w, h) }
    }

    pub fn translate(&mut self, tx: f64, ty: f64) {
        unsafe { ffi::qtc_qpainter_translate(self.0, tx, ty) }
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        unsafe { ffi::qtc_qpainter_scale(self.0, sx, sy) }
    }

    pub fn get_transform(&self) -> Transform {
        unsafe { Transform(ffi::qtc_qpainter_get_transform(self.0)) }
    }

    pub fn set_transform(&mut self, ts: &Transform) {
        unsafe { ffi::qtc_qpainter_set_transform(self.0, ts.0, false) }
    }

    pub fn apply_transform(&mut self, ts: &Transform) {
        unsafe { ffi::qtc_qpainter_set_transform(self.0, ts.0, true) }
    }

    pub fn set_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        unsafe { ffi::qtc_qpainter_set_clip_rect(self.0, x, y, w, h) }
    }

    pub fn set_clip_path(&mut self, path: &PainterPath) {
        unsafe { ffi::qtc_qpainter_set_clip_path(self.0, path.0) }
    }

    pub fn reset_clip_path(&mut self) {
        unsafe { ffi::qtc_qpainter_reset_clip_path(self.0) }
    }

    pub fn set_composition_mode(&mut self, mode: CompositionMode) {
        unsafe { ffi::qtc_qpainter_set_composition_mode(self.0, mode as ffi::CompositionMode) }
    }

    pub fn end(&mut self) {
        unsafe { ffi::qtc_qpainter_end(self.0) }
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        if self.1 {
            unsafe { ffi::qtc_qpainter_destroy(self.0) }
        }
    }
}


pub struct PainterPath(*mut ffi::qtc_qpainterpath);

impl PainterPath {
    pub fn new() -> PainterPath {
        unsafe { PainterPath(ffi::qtc_qpainterpath_create()) }
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        unsafe { ffi::qtc_qpainterpath_move_to(self.0, x, y) }
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        unsafe { ffi::qtc_qpainterpath_line_to(self.0, x, y) }
    }

    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        unsafe { ffi::qtc_qpainterpath_curve_to(self.0, x1, y1, x2, y2, x, y) }
    }

    pub fn close_path(&mut self) {
        unsafe { ffi::qtc_qpainterpath_close_path(self.0) }
    }

    pub fn add_text(&mut self, x: f64, y: f64, font: &Font, text: &str) {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::qtc_qpainterpath_add_text(self.0, x, y, font.0, c_text.as_ptr()) }
    }

    pub fn len(&self) -> i32 {
        unsafe { ffi::qtc_qpainterpath_element_count(self.0) }
    }

    pub fn get(&self, index: i32) -> (PathSegmentType, f64, f64) {
        unsafe {
            let seg = ffi::qtc_qpainterpath_element_at(self.0, index);
            let kind = match seg.kind {
                ffi::PathSegmentType_MoveToSegment => PathSegmentType::MoveTo,
                ffi::PathSegmentType_LineToSegment => PathSegmentType::LineTo,
                ffi::PathSegmentType_CurveToSegment => PathSegmentType::CurveTo,
                _ => unreachable!(),
            };
            (kind, seg.x, seg.y)
        }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let rect = unsafe { ffi::qtc_qpainterpath_get_bbox(self.0) };

        (rect.x, rect.y, rect.w, rect.h)
    }

    pub fn set_fill_rule(&mut self, rule: FillRule) {
        unsafe { ffi::qtc_qpainterpath_set_fill_rule(self.0, rule as ffi::FillRule) }
    }
}

impl Drop for PainterPath {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qpainterpath_destroy(self.0) }
    }
}


pub struct Transform(*mut ffi::qtc_qtransform);

impl Transform {
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Transform {
        unsafe { Transform(ffi::qtc_qtransform_create_from(a, b, c, d, e, f)) }
    }

    pub fn data(&self) -> (f64, f64, f64, f64, f64, f64) {
        let ts = unsafe { ffi::qtc_qtransform_get_data(self.0) };
        (ts.a, ts.b, ts.c, ts.d, ts.e, ts.f)
    }
}

impl Default for Transform {
    fn default() -> Transform {
        unsafe { Transform(ffi::qtc_qtransform_create()) }
    }
}

impl Drop for Transform {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qtransform_destroy(self.0) }
    }
}


pub struct Pen(*mut ffi::qtc_qpen);

impl Pen {
    pub fn new() -> Pen {
        unsafe { Pen(ffi::qtc_qpen_create()) }
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::qtc_qpen_set_color(self.0, r, g, b, a) }
    }

    pub fn set_brush(&mut self, brush: Brush) {
        unsafe { ffi::qtc_qpen_set_brush(self.0, brush.0) }
    }

    pub fn set_line_cap(&mut self, s: LineCap) {
        unsafe { ffi::qtc_qpen_set_line_cap(self.0, s as ffi::PenCapStyle) }
    }

    pub fn set_line_join(&mut self, s: LineJoin) {
        unsafe { ffi::qtc_qpen_set_line_join(self.0, s as ffi::PenJoinStyle) }
    }

    pub fn set_width(&mut self, width: f64) {
        unsafe { ffi::qtc_qpen_set_width(self.0, width) }
    }

    pub fn set_miter_limit(&mut self, limit: f64) {
        unsafe { ffi::qtc_qpen_set_miter_limit(self.0, limit) }
    }

    pub fn set_dash_offset(&mut self, offset: f64) {
        unsafe { ffi::qtc_qpen_set_dash_offset(self.0, offset) }
    }

    pub fn set_dash_array(&mut self, offset: &[f64]) {
        assert!(offset.len() < i32::MAX as usize);
        unsafe { ffi::qtc_qpen_set_dash_array(self.0, offset.as_ptr(), offset.len() as i32) }
    }
}

impl Drop for Pen {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qpen_destroy(self.0) }
    }
}


pub struct Brush(*mut ffi::qtc_qbrush);

impl Brush {
    pub fn new() -> Brush {
        unsafe { Brush(ffi::qtc_qbrush_create()) }
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::qtc_qbrush_set_color(self.0, r, g, b, a) }
    }

    pub fn set_linear_gradient(&mut self, lg: LinearGradient) {
        unsafe { ffi::qtc_qbrush_set_linear_gradient(self.0, lg.0) }
    }

    pub fn set_radial_gradient(&mut self, rg: RadialGradient) {
        unsafe { ffi::qtc_qbrush_set_radial_gradient(self.0, rg.0) }
    }

    pub fn set_pattern(&mut self, img: Image) {
        unsafe { ffi::qtc_qbrush_set_pattern(self.0, img.0) }
    }

    pub fn set_transform(&mut self, ts: Transform) {
        unsafe { ffi::qtc_qbrush_set_transform(self.0, ts.0) }
    }
}

impl Drop for Brush {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qbrush_destroy(self.0) }
    }
}


pub trait Gradient {
    fn set_color_at(&mut self, offset: f64, r: u8, g: u8, b: u8, a: u8);
    fn set_spread(&mut self, spread: Spread);
}


pub struct LinearGradient(*mut ffi::qtc_qlineargradient);

impl LinearGradient {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> LinearGradient {
        unsafe { LinearGradient(ffi::qtc_qlineargradient_create(x1, y1, x2, y2)) }
    }
}

impl Gradient for LinearGradient {
    fn set_color_at(&mut self, offset: f64, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::qtc_qlineargradient_set_color_at(self.0, offset, r, g, b, a) }
    }

    fn set_spread(&mut self, spread: Spread) {
        unsafe { ffi::qtc_qlineargradient_set_spread(self.0, spread as ffi::Spread) }
    }
}

impl Drop for LinearGradient {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qlineargradient_destroy(self.0) }
    }
}


pub struct RadialGradient(*mut ffi::qtc_qradialgradient);

impl RadialGradient {
    pub fn new(cx: f64, cy: f64, fx: f64, fy: f64, r: f64) -> RadialGradient {
        unsafe { RadialGradient(ffi::qtc_qradialgradient_create(cx, cy, fx, fy, r)) }
    }
}

impl Gradient for RadialGradient {
    fn set_color_at(&mut self, offset: f64, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::qtc_qradialgradient_set_color_at(self.0, offset, r, g, b, a) }
    }

    fn set_spread(&mut self, spread: Spread) {
        unsafe { ffi::qtc_qradialgradient_set_spread(self.0, spread as ffi::Spread) }
    }
}

impl Drop for RadialGradient {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qradialgradient_destroy(self.0) }
    }
}


pub struct Font(*mut ffi::qtc_qfont);

impl Font {
    pub fn new() -> Font {
        unsafe { Font(ffi::qtc_qfont_create()) }
    }

    pub fn set_family(&mut self, family: &str) {
        let c_family = CString::new(family).unwrap();
        unsafe { ffi::qtc_qfont_set_family(self.0, c_family.as_ptr()); }
    }

    pub fn set_style(&mut self, style: FontStyle) {
        unsafe { ffi::qtc_qfont_set_style(self.0, style as ffi::FontStyle); }
    }

    pub fn set_small_caps(&mut self, flag: bool) {
        unsafe { ffi::qtc_qfont_set_small_caps(self.0, flag); }
    }

    pub fn set_weight(&mut self, weight: FontWeight) {
        unsafe { ffi::qtc_qfont_set_weight(self.0, weight as ffi::FontWeight); }
    }

    pub fn set_stretch(&mut self, stretch: FontStretch) {
        unsafe { ffi::qtc_qfont_set_stretch(self.0, stretch as ffi::FontStretch); }
    }

    pub fn set_size(&mut self, size: f64) {
        unsafe { ffi::qtc_qfont_set_size(self.0, size); }
    }

    pub fn set_letter_spacing(&mut self, size: f64) {
        unsafe { ffi::qtc_qfont_set_letter_spacing(self.0, size); }
    }

    pub fn set_word_spacing(&mut self, size: f64) {
        unsafe { ffi::qtc_qfont_set_word_spacing(self.0, size); }
    }

    pub fn print_debug(&self) {
        unsafe { ffi::qtc_qfont_print_debug(self.0); }
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        unsafe { Font(ffi::qtc_qfont_clone(self.0)) }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qfont_destroy(self.0) }
    }
}


pub struct FontMetricsF(*mut ffi::qtc_qfontmetricsf);

impl FontMetricsF {
    pub fn height(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_height(self.0) }
    }

    pub fn width(&self, text: &str) -> f64 {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::qtc_qfontmetricsf_width(self.0, c_text.as_ptr()) }
    }

    pub fn ascent(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_get_ascent(self.0) }
    }

    pub fn underline_pos(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_get_underline_pos(self.0) }
    }

    pub fn overline_pos(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_get_overline_pos(self.0) }
    }

    pub fn strikeout_pos(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_get_strikeout_pos(self.0) }
    }

    pub fn line_width(&self) -> f64 {
        unsafe { ffi::qtc_qfontmetricsf_get_line_width(self.0) }
    }

    pub fn full_width(&self, text: &str) -> f64 {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::qtc_qfontmetricsf_full_width(self.0, c_text.as_ptr()) }
    }

    pub fn bounding_box(&self, text: &str) -> (f64, f64, f64, f64) {
        let c_text = CString::new(text).unwrap();
        let rect = unsafe { ffi::qtc_qfontmetricsf_get_bbox(self.0, c_text.as_ptr()) };

        (rect.x, rect.y, rect.w, rect.h)
    }
}

impl Drop for FontMetricsF {
    fn drop(&mut self) {
        unsafe { ffi::qtc_qfontmetricsf_destroy(self.0) }
    }
}
