use std::ffi::CString;
use std::i32;
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
pub enum Spread {
    Pad = ffi::Spread_PadSpread as isize,
    Reflect = ffi::Spread_ReflectSpread as isize,
    Repeat = ffi::Spread_RepeatSpread as isize,
}


pub struct Image(*mut ffi::qtc_qimage);

impl Image {
    pub fn new_rgba(width: u32, height: u32) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_create_rgba(width, height)) }
    }

    pub fn new_rgba_premultiplied(width: u32, height: u32) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_create_rgba_premultiplied(width, height)) }
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

    pub fn to_rgba(&self) -> Option<Image> {
        unsafe { Self::from_ptr(ffi::qtc_qimage_to_rgba(self.0)) }
    }

    pub fn save(&self, path: &str) -> bool {
        let c_path = CString::new(path).unwrap();
        unsafe { ffi::qtc_qimage_save(self.0, c_path.as_ptr()) }
    }

    pub fn resize(
        &self,
        width: u32,
        height: u32,
        ratio: AspectRatioMode,
        smooth: bool,
    ) -> Option<Image> {
        unsafe {
            Self::from_ptr(ffi::qtc_qimage_resize(
                self.0, width, height, ratio as ffi::AspectRatioMode, smooth
            ))
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
            let len = ffi::qtc_qimage_get_size_in_bytes(self.0) as usize;

            ImageData {
                slice: slice::from_raw_parts_mut(ptr, len),
            }
        }
    }

    pub fn data_mut(&mut self) -> ImageData {
        unsafe {
            let ptr = ffi::qtc_qimage_get_data(self.0);
            let len = ffi::qtc_qimage_get_size_in_bytes(self.0) as usize;

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

    pub fn set_antialiasing(&self, flag: bool) {
        unsafe { ffi::qtc_qpainter_set_antialiasing(self.0, flag); }
    }

    pub fn set_smooth_pixmap_transform(&self, flag: bool) {
        unsafe { ffi::qtc_qpainter_set_smooth_pixmap_transform(self.0, flag); }
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

    pub fn draw_image_rect(&mut self, x: f64, y: f64, w: f64, h: f64, img: &Image) {
        unsafe { ffi::qtc_qpainter_draw_image_rect(self.0, x, y, w, h, img.0) }
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
