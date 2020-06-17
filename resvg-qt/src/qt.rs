// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ffi::CString;
use std::i32;
use std::ops::{Deref, DerefMut};
use std::slice;

mod ffi {
    #![allow(non_upper_case_globals)]

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qimage {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qpainter {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qpainterpath {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qtransform {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qpen {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qbrush {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qlineargradient {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_qradialgradient {
        _unused: [u8; 0],
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct qtc_transform {
        pub a: f64,
        pub b: f64,
        pub c: f64,
        pub d: f64,
        pub e: f64,
        pub f: f64,
    }
    pub const PenCapStyle_FlatCap: PenCapStyle = 0;
    pub const PenCapStyle_SquareCap: PenCapStyle = 16;
    pub const PenCapStyle_RoundCap: PenCapStyle = 32;
    pub type PenCapStyle = u32;
    pub const PenJoinStyle_BevelJoin: PenJoinStyle = 64;
    pub const PenJoinStyle_RoundJoin: PenJoinStyle = 128;
    pub const PenJoinStyle_MiterJoin: PenJoinStyle = 256;
    pub type PenJoinStyle = u32;
    pub const FillRule_OddEvenFill: FillRule = 0;
    pub const FillRule_WindingFill: FillRule = 1;
    pub type FillRule = u32;
    pub const Spread_PadSpread: Spread = 0;
    pub const Spread_ReflectSpread: Spread = 1;
    pub const Spread_RepeatSpread: Spread = 2;
    pub type Spread = u32;
    pub const CompositionMode_CompositionMode_SourceOver: CompositionMode = 0;
    pub const CompositionMode_CompositionMode_DestinationOver: CompositionMode = 1;
    pub const CompositionMode_CompositionMode_Clear: CompositionMode = 2;
    pub const CompositionMode_CompositionMode_Source: CompositionMode = 3;
    pub const CompositionMode_CompositionMode_Destination: CompositionMode = 4;
    pub const CompositionMode_CompositionMode_SourceIn: CompositionMode = 5;
    pub const CompositionMode_CompositionMode_DestinationIn: CompositionMode = 6;
    pub const CompositionMode_CompositionMode_SourceOut: CompositionMode = 7;
    pub const CompositionMode_CompositionMode_DestinationOut: CompositionMode = 8;
    pub const CompositionMode_CompositionMode_SourceAtop: CompositionMode = 9;
    pub const CompositionMode_CompositionMode_DestinationAtop: CompositionMode = 10;
    pub const CompositionMode_CompositionMode_Xor: CompositionMode = 11;
    pub const CompositionMode_CompositionMode_Plus: CompositionMode = 12;
    pub const CompositionMode_CompositionMode_Multiply: CompositionMode = 13;
    pub const CompositionMode_CompositionMode_Screen: CompositionMode = 14;
    pub const CompositionMode_CompositionMode_Overlay: CompositionMode = 15;
    pub const CompositionMode_CompositionMode_Darken: CompositionMode = 16;
    pub const CompositionMode_CompositionMode_Lighten: CompositionMode = 17;
    pub const CompositionMode_CompositionMode_ColorDodge: CompositionMode = 18;
    pub const CompositionMode_CompositionMode_ColorBurn: CompositionMode = 19;
    pub const CompositionMode_CompositionMode_HardLight: CompositionMode = 20;
    pub const CompositionMode_CompositionMode_SoftLight: CompositionMode = 21;
    pub const CompositionMode_CompositionMode_Difference: CompositionMode = 22;
    pub const CompositionMode_CompositionMode_Exclusion: CompositionMode = 23;
    pub type CompositionMode = u32;
    pub const AspectRatioMode_IgnoreAspectRatio: AspectRatioMode = 0;
    pub const AspectRatioMode_KeepAspectRatio: AspectRatioMode = 1;
    pub const AspectRatioMode_KeepAspectRatioByExpanding: AspectRatioMode = 2;
    pub type AspectRatioMode = u32;
    extern "C" {
        pub fn qtc_qimage_create_rgba_premultiplied(width: u32, height: u32) -> *mut qtc_qimage;
        pub fn qtc_qimage_create_rgba(width: u32, height: u32) -> *mut qtc_qimage;
        pub fn qtc_qimage_get_data(c_img: *mut qtc_qimage) -> *mut u8;
        pub fn qtc_qimage_get_size_in_bytes(c_img: *mut qtc_qimage) -> u32;
        pub fn qtc_qimage_resize(
            c_img: *mut qtc_qimage,
            width: u32,
            height: u32,
            ratio: AspectRatioMode,
            smooth_transformation: bool,
        ) -> *mut qtc_qimage;
        pub fn qtc_qimage_copy(
            c_img: *mut qtc_qimage,
            x: u32,
            y: u32,
            width: u32,
            height: u32,
        ) -> *mut qtc_qimage;
        pub fn qtc_qimage_fill(c_img: *mut qtc_qimage, r: u8, g: u8, b: u8, a: u8);
        pub fn qtc_qimage_to_rgba(c_img: *mut qtc_qimage) -> *mut qtc_qimage;
        pub fn qtc_qimage_get_width(c_img: *mut qtc_qimage) -> u32;
        pub fn qtc_qimage_get_height(c_img: *mut qtc_qimage) -> u32;
        pub fn qtc_qimage_save(c_img: *mut qtc_qimage, path: *const ::std::os::raw::c_char) -> bool;
        pub fn qtc_qimage_destroy(c_img: *mut qtc_qimage);
        pub fn qtc_qpainter_create(c_img: *mut qtc_qimage) -> *mut qtc_qpainter;
        pub fn qtc_qpainter_set_antialiasing(c_p: *mut qtc_qpainter, flag: bool);
        pub fn qtc_qpainter_set_smooth_pixmap_transform(c_p: *mut qtc_qpainter, flag: bool);
        pub fn qtc_qpainter_set_pen(c_p: *mut qtc_qpainter, c_pen: *mut qtc_qpen);
        pub fn qtc_qpainter_reset_pen(c_p: *mut qtc_qpainter);
        pub fn qtc_qpainter_set_brush(c_p: *mut qtc_qpainter, c_brush: *mut qtc_qbrush);
        pub fn qtc_qpainter_reset_brush(c_p: *mut qtc_qpainter);
        pub fn qtc_qpainter_set_opacity(c_p: *mut qtc_qpainter, opacity: f64);
        pub fn qtc_qpainter_draw_path(c_p: *mut qtc_qpainter, c_pp: *mut qtc_qpainterpath);
        pub fn qtc_qpainter_draw_image(c_p: *mut qtc_qpainter, x: f64, y: f64, c_img: *mut qtc_qimage);
        pub fn qtc_qpainter_draw_image_rect(
            c_p: *mut qtc_qpainter,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            c_img: *mut qtc_qimage,
        );
        pub fn qtc_qpainter_draw_rect(c_p: *mut qtc_qpainter, x: f64, y: f64, w: f64, h: f64);
        pub fn qtc_qpainter_translate(c_p: *mut qtc_qpainter, tx: f64, ty: f64);
        pub fn qtc_qpainter_scale(c_p: *mut qtc_qpainter, sx: f64, sy: f64);
        pub fn qtc_qpainter_get_transform(c_p: *mut qtc_qpainter) -> *mut qtc_qtransform;
        pub fn qtc_qpainter_set_transform(
            c_p: *mut qtc_qpainter,
            q_ts: *mut qtc_qtransform,
            combine: bool,
        );
        pub fn qtc_qpainter_set_clip_rect(c_p: *mut qtc_qpainter, x: f64, y: f64, w: f64, h: f64);
        pub fn qtc_qpainter_reset_clip_path(c_p: *mut qtc_qpainter);
        pub fn qtc_qpainter_set_composition_mode(c_p: *mut qtc_qpainter, mode: CompositionMode);
        pub fn qtc_qpainter_end(c_p: *mut qtc_qpainter);
        pub fn qtc_qpainter_destroy(c_p: *mut qtc_qpainter);
        pub fn qtc_qpainterpath_create() -> *mut qtc_qpainterpath;
        pub fn qtc_qpainterpath_move_to(c_pp: *mut qtc_qpainterpath, x: f64, y: f64);
        pub fn qtc_qpainterpath_line_to(c_pp: *mut qtc_qpainterpath, x: f64, y: f64);
        pub fn qtc_qpainterpath_curve_to(
            c_pp: *mut qtc_qpainterpath,
            x1: f64,
            y1: f64,
            x2: f64,
            y2: f64,
            x: f64,
            y: f64,
        );
        pub fn qtc_qpainterpath_close_path(c_pp: *mut qtc_qpainterpath);
        pub fn qtc_qpainterpath_set_fill_rule(c_pp: *mut qtc_qpainterpath, rule: FillRule);
        pub fn qtc_qpainterpath_destroy(c_pp: *mut qtc_qpainterpath);
        pub fn qtc_qtransform_create() -> *mut qtc_qtransform;
        pub fn qtc_qtransform_create_from(
            a: f64,
            b: f64,
            c: f64,
            d: f64,
            e: f64,
            f: f64,
        ) -> *mut qtc_qtransform;
        pub fn qtc_qtransform_get_data(c_ts: *mut qtc_qtransform) -> qtc_transform;
        pub fn qtc_qtransform_destroy(c_ts: *mut qtc_qtransform);
        pub fn qtc_qpen_create() -> *mut qtc_qpen;
        pub fn qtc_qpen_set_color(c_pen: *mut qtc_qpen, r: u8, g: u8, b: u8, a: u8);
        pub fn qtc_qpen_set_brush(c_pen: *mut qtc_qpen, c_brush: *mut qtc_qbrush);
        pub fn qtc_qpen_set_line_cap(c_pen: *mut qtc_qpen, s: PenCapStyle);
        pub fn qtc_qpen_set_line_join(c_pen: *mut qtc_qpen, s: PenJoinStyle);
        pub fn qtc_qpen_set_width(c_pen: *mut qtc_qpen, width: f64);
        pub fn qtc_qpen_set_miter_limit(c_pen: *mut qtc_qpen, limit: f64);
        pub fn qtc_qpen_set_dash_offset(c_pen: *mut qtc_qpen, offset: f64);
        pub fn qtc_qpen_set_dash_array(
            c_pen: *mut qtc_qpen,
            array: *const f64,
            len: ::std::os::raw::c_int,
        );
        pub fn qtc_qpen_destroy(c_pen: *mut qtc_qpen);
        pub fn qtc_qbrush_create() -> *mut qtc_qbrush;
        pub fn qtc_qbrush_set_color(c_brush: *mut qtc_qbrush, r: u8, g: u8, b: u8, a: u8);
        pub fn qtc_qbrush_set_linear_gradient(c_brush: *mut qtc_qbrush, c_lg: *mut qtc_qlineargradient);
        pub fn qtc_qbrush_set_radial_gradient(c_brush: *mut qtc_qbrush, c_rg: *mut qtc_qradialgradient);
        pub fn qtc_qbrush_set_pattern(c_brush: *mut qtc_qbrush, c_img: *mut qtc_qimage);
        pub fn qtc_qbrush_set_transform(c_brush: *mut qtc_qbrush, c_ts: *mut qtc_qtransform);
        pub fn qtc_qbrush_destroy(c_brush: *mut qtc_qbrush);
        pub fn qtc_qlineargradient_create(
            x1: f64,
            y1: f64,
            x2: f64,
            y2: f64,
        ) -> *mut qtc_qlineargradient;
        pub fn qtc_qlineargradient_set_color_at(
            c_lg: *mut qtc_qlineargradient,
            offset: f64,
            r: u8,
            g: u8,
            b: u8,
            a: u8,
        );
        pub fn qtc_qlineargradient_set_spread(c_lg: *mut qtc_qlineargradient, s: Spread);
        pub fn qtc_qlineargradient_destroy(c_lg: *mut qtc_qlineargradient);
        pub fn qtc_qradialgradient_create(
            cx: f64,
            cy: f64,
            fx: f64,
            fy: f64,
            r: f64,
        ) -> *mut qtc_qradialgradient;
        pub fn qtc_qradialgradient_set_color_at(
            c_rg: *mut qtc_qradialgradient,
            offset: f64,
            r: u8,
            g: u8,
            b: u8,
            a: u8,
        );
        pub fn qtc_qradialgradient_set_spread(c_rg: *mut qtc_qradialgradient, s: Spread);
        pub fn qtc_qradialgradient_destroy(c_rg: *mut qtc_qradialgradient);
    }
}


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

#[allow(dead_code)]
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
