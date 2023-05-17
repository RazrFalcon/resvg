// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/// A 2D integer size representation.
///
/// Width and height are guarantee to be > 0.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct IntSize {
    width: u32,
    height: u32,
}

impl IntSize {
    /// Creates a new `IntSize` from values.
    #[inline]
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(IntSize { width, height })
        } else {
            None
        }
    }

    /// Creates a new `IntSize` from [`usvg::Size`].
    #[inline]
    pub fn from_usvg(size: usvg::Size) -> Self {
        IntSize::new(
            std::cmp::max(1, size.width().round() as u32),
            std::cmp::max(1, size.height().round() as u32),
        )
        .unwrap()
    }

    /// Returns width.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns height.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns width and height as a tuple.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Scales current size by the specified factor.
    #[inline]
    pub fn scale_by(&self, factor: f64) -> Option<Self> {
        Self::new(
            (self.width as f64 * factor).round() as u32,
            (self.height as f64 * factor).round() as u32,
        )
    }

    /// Scales current size to the specified size.
    #[inline]
    pub fn scale_to(&self, to: Self) -> Self {
        size_scale(*self, to, false)
    }

    /// Scales current size to the specified width.
    #[inline]
    pub fn scale_to_width(&self, new_width: u32) -> Option<Self> {
        let new_height = (new_width as f32 * self.height as f32 / self.width as f32).ceil();
        Self::new(new_width, new_height as u32)
    }

    /// Scales current size to the specified height.
    #[inline]
    pub fn scale_to_height(&self, new_height: u32) -> Option<Self> {
        let new_width = (new_height as f32 * self.width as f32 / self.height as f32).ceil();
        Self::new(new_width as u32, new_height)
    }

    /// Expands current size to the specified size.
    #[inline]
    pub fn expand_to(&self, to: Self) -> Self {
        size_scale(*self, to, true)
    }

    /// Fits size into a viewbox.
    pub fn fit_view_box(&self, vb: &usvg::ViewBox) -> Self {
        let s = IntSize::from_usvg(vb.rect.size());

        if vb.aspect.align == usvg::Align::None {
            s
        } else {
            if vb.aspect.slice {
                self.expand_to(s)
            } else {
                self.scale_to(s)
            }
        }
    }

    /// Converts the current `IntSize` to `Size`.
    #[inline]
    pub fn to_size(&self) -> usvg::Size {
        // Can't fail, because `IntSize` is always valid.
        usvg::Size::new(self.width as f64, self.height as f64).unwrap()
    }

    /// Converts the current `IntSize` to `IntRect`.
    #[inline]
    pub fn to_int_rect(&self) -> IntRect {
        // Can't fail, because `IntSize` is always valid.
        IntRect::new(0, 0, self.width, self.height).unwrap()
    }
}

impl std::fmt::Debug for IntSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "IntSize({} {})", self.width, self.height)
    }
}

impl std::fmt::Display for IntSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn size_scale(s1: IntSize, s2: IntSize, expand: bool) -> IntSize {
    let rw = (s2.height as f64 * s1.width as f64 / s1.height as f64).ceil() as u32;
    let with_h = if expand {
        rw <= s2.width
    } else {
        rw >= s2.width
    };
    if !with_h {
        IntSize::new(rw, s2.height).unwrap()
    } else {
        let h = (s2.width as f64 * s1.height as f64 / s1.width as f64).ceil() as u32;
        IntSize::new(s2.width, h).unwrap()
    }
}

/// A 2D integer rect representation.
///
/// Width and height are guarantee to be > 0.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct IntRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl IntRect {
    /// Creates a new `Rect` from values.
    #[inline]
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(IntRect {
                x,
                y,
                width,
                height,
            })
        } else {
            None
        }
    }

    /// Creates a new `IntRect` from [`usvg::Rect`].
    #[inline]
    pub fn from_usvg(rect: usvg::Rect) -> Self {
        IntRect::new(
            rect.x() as i32,
            rect.y() as i32,
            std::cmp::max(1, rect.width().round() as u32),
            std::cmp::max(1, rect.height().round() as u32),
        )
        .unwrap()
    }

    /// Returns rect's size.
    #[inline]
    pub fn size(&self) -> IntSize {
        // Can't fail, because `IntSize` is always valid.
        IntSize::new(self.width, self.height).unwrap()
    }

    /// Returns rect's X position.
    #[inline]
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Returns rect's Y position.
    #[inline]
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Returns rect's width.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns rect's height.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns rect's left edge position.
    #[inline]
    pub fn left(&self) -> i32 {
        self.x
    }

    /// Returns rect's right edge position.
    #[inline]
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Returns rect's top edge position.
    #[inline]
    pub fn top(&self) -> i32 {
        self.y
    }

    /// Returns rect's bottom edge position.
    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Translates the rect by the specified offset.
    #[inline]
    pub fn translate(&self, tx: i32, ty: i32) -> Self {
        IntRect {
            x: self.x + tx,
            y: self.y + ty,
            width: self.width,
            height: self.height,
        }
    }

    /// Translates the rect to the specified position.
    #[inline]
    pub fn translate_to(&self, x: i32, y: i32) -> Self {
        IntRect {
            x,
            y,
            width: self.width,
            height: self.height,
        }
    }

    /// Checks that rect contains a point.
    #[inline]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if x < self.x || x > self.x + self.width as i32 - 1 {
            return false;
        }

        if y < self.y || y > self.y + self.height as i32 - 1 {
            return false;
        }

        true
    }

    /// Fits the current rect into the specified bounds.
    #[inline]
    pub fn fit_to_rect(&self, bounds: IntRect) -> Self {
        let mut r = *self;

        if r.x < bounds.x() {
            r.x = bounds.x();
        }
        if r.y < bounds.y() {
            r.y = bounds.y();
        }

        if r.right() > bounds.width as i32 {
            r.width = std::cmp::max(1, bounds.width as i32 - r.x) as u32;
        }

        if r.bottom() > bounds.height as i32 {
            r.height = std::cmp::max(1, bounds.height as i32 - r.y) as u32;
        }

        r
    }

    /// Converts into `Rect`.
    #[inline]
    pub fn to_rect(&self) -> usvg::Rect {
        // Can't fail, because `IntRect` is always valid.
        usvg::Rect::new(
            self.x as f64,
            self.y as f64,
            self.width as f64,
            self.height as f64,
        )
        .unwrap()
    }

    /// Converts into `PathBbox`.
    #[inline]
    pub fn to_path_bbox(&self) -> usvg::PathBbox {
        // Can't fail, because `IntRect` is always valid.
        usvg::PathBbox::new(
            self.x as f64,
            self.y as f64,
            self.width as f64,
            self.height as f64,
        )
        .unwrap()
    }
}

impl std::fmt::Debug for IntRect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "IntRect({} {} {} {})",
            self.x, self.y, self.width, self.height
        )
    }
}

impl std::fmt::Display for IntRect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Converts `viewBox` to `Transform` with an optional clip rectangle.
///
/// Unlike `view_box_to_transform`, returns an optional clip rectangle
/// that should be applied before rendering the image.
pub fn view_box_to_transform_with_clip(
    view_box: &usvg::ViewBox,
    img_size: IntSize,
) -> (usvg::Transform, Option<usvg::Rect>) {
    let r = view_box.rect;

    let new_size = img_size.fit_view_box(view_box);

    let (tx, ty, clip) = if view_box.aspect.slice {
        let (dx, dy) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            0.0,
            0.0,
            new_size.width() as f64 - r.width(),
            new_size.height() as f64 - r.height(),
        );

        (r.x() - dx, r.y() - dy, Some(r))
    } else {
        let (dx, dy) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            r.x(),
            r.y(),
            r.width() - new_size.width() as f64,
            r.height() - new_size.height() as f64,
        );

        (dx, dy, None)
    };

    let sx = new_size.width() as f64 / img_size.width() as f64;
    let sy = new_size.height() as f64 / img_size.height() as f64;
    let ts = usvg::Transform::new(sx, 0.0, 0.0, sy, tx, ty);

    (ts, clip)
}

pub trait UsvgRectExt {
    fn to_int_rect_round_out(&self) -> IntRect;
    fn to_skia_rect(&self) -> Option<tiny_skia::Rect>;
}

impl UsvgRectExt for usvg::Rect {
    fn to_int_rect_round_out(&self) -> IntRect {
        IntRect::new(
            self.x().floor() as i32,
            self.y().floor() as i32,
            std::cmp::max(1, self.width().ceil() as u32),
            std::cmp::max(1, self.height().ceil() as u32),
        )
        .unwrap()
    }

    fn to_skia_rect(&self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_xywh(
            self.x() as f32,
            self.y() as f32,
            self.width() as f32,
            self.height() as f32,
        )
    }
}
