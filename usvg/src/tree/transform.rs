use crate::FuzzyEq;

/// Representation of the [`<transform>`] type.
///
/// [`<transform>`]: https://www.w3.org/TR/SVG2/coords.html#InterfaceSVGTransform
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl From<svgtypes::Transform> for Transform {
    fn from(ts: svgtypes::Transform) -> Self {
        Transform::new(ts.a, ts.b, ts.c, ts.d, ts.e, ts.f)
    }
}

impl Transform {
    /// Constructs a new transform.
    #[inline]
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Transform { a, b, c, d, e, f, }
    }

    /// Constructs a new translate transform.
    #[inline]
    pub fn new_translate(x: f64, y: f64) -> Self {
        Transform::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Constructs a new scale transform.
    #[inline]
    pub fn new_scale(sx: f64, sy: f64) -> Self {
        Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Constructs a new rotate transform.
    #[inline]
    pub fn new_rotate(angle: f64) -> Self {
        let v = angle.to_radians();
        let a =  v.cos();
        let b =  v.sin();
        let c = -b;
        let d =  a;
        Transform::new(a, b, c, d, 0.0, 0.0)
    }

    /// Constructs a new rotate transform at the specified position.
    #[inline]
    pub fn new_rotate_at(angle: f64, x: f64, y: f64) -> Self {
        let mut ts = Self::default();
        ts.translate(x, y);
        ts.rotate(angle);
        ts.translate(-x, -y);
        ts
    }

    /// Constructs a new skew transform along then X axis.
    #[inline]
    pub fn new_skew_x(angle: f64) -> Self {
        let c = angle.to_radians().tan();
        Transform::new(1.0, 0.0, c, 1.0, 0.0, 0.0)
    }

    /// Constructs a new skew transform along then Y axis.
    #[inline]
    pub fn new_skew_y(angle: f64) -> Self {
        let b = angle.to_radians().tan();
        Transform::new(1.0, b, 0.0, 1.0, 0.0, 0.0)
    }

    /// Translates the current transform.
    #[inline]
    pub fn translate(&mut self, x: f64, y: f64) {
        self.append(&Transform::new_translate(x, y));
    }

    /// Scales the current transform.
    #[inline]
    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.append(&Transform::new_scale(sx, sy));
    }

    /// Rotates the current transform.
    #[inline]
    pub fn rotate(&mut self, angle: f64) {
        self.append(&Transform::new_rotate(angle));
    }

    /// Rotates the current transform at the specified position.
    #[inline]
    pub fn rotate_at(&mut self, angle: f64, x: f64, y: f64) {
        self.translate(x, y);
        self.rotate(angle);
        self.translate(-x, -y);
    }

    /// Skews the current transform along the X axis.
    #[inline]
    pub fn skew_x(&mut self, angle: f64) {
        self.append(&Transform::new_skew_x(angle));
    }

    /// Skews the current transform along the Y axis.
    #[inline]
    pub fn skew_y(&mut self, angle: f64) {
        self.append(&Transform::new_skew_y(angle));
    }

    /// Appends transform to the current transform.
    #[inline]
    pub fn append(&mut self, other: &Transform) {
        let ts = multiply(self, other);
        self.a = ts.a;
        self.b = ts.b;
        self.c = ts.c;
        self.d = ts.d;
        self.e = ts.e;
        self.f = ts.f;
    }

    /// Prepends transform to the current transform.
    #[inline]
    pub fn prepend(&mut self, other: &Transform) {
        let ts = multiply(other, self);
        self.a = ts.a;
        self.b = ts.b;
        self.c = ts.c;
        self.d = ts.d;
        self.e = ts.e;
        self.f = ts.f;
    }

    /// Returns `true` if the transform is default, aka `(1 0 0 1 0 0)`.
    pub fn is_default(&self) -> bool {
           self.a.fuzzy_eq(&1.0)
        && self.b.fuzzy_eq(&0.0)
        && self.c.fuzzy_eq(&0.0)
        && self.d.fuzzy_eq(&1.0)
        && self.e.fuzzy_eq(&0.0)
        && self.f.fuzzy_eq(&0.0)
    }

    /// Returns `true` if the transform contains only translate part, aka `(1 0 0 1 x y)`.
    pub fn is_translate(&self) -> bool {
           self.a.fuzzy_eq(&1.0)
        && self.b.fuzzy_eq(&0.0)
        && self.c.fuzzy_eq(&0.0)
        && self.d.fuzzy_eq(&1.0)
        && (self.e.fuzzy_ne(&0.0) || self.f.fuzzy_ne(&0.0))
    }

    /// Returns `true` if the transform contains only scale part, aka `(sx 0 0 sy 0 0)`.
    pub fn is_scale(&self) -> bool {
          (self.a.fuzzy_ne(&1.0) || self.d.fuzzy_ne(&1.0))
        && self.b.fuzzy_eq(&0.0)
        && self.c.fuzzy_eq(&0.0)
        && self.e.fuzzy_eq(&0.0)
        && self.f.fuzzy_eq(&0.0)
    }

    /// Returns `true` if the transform contains translate part.
    pub fn has_translate(&self) -> bool {
        self.e.fuzzy_ne(&0.0) || self.f.fuzzy_ne(&0.0)
    }

    /// Returns `true` if the transform contains scale part.
    pub fn has_scale(&self) -> bool {
        let (sx, sy) = self.get_scale();
        sx.fuzzy_ne(&1.0) || sy.fuzzy_ne(&1.0)
    }

    /// Returns `true` if the transform scale is proportional.
    ///
    /// The proportional scale is when `<sx>` equal to `<sy>`.
    pub fn has_proportional_scale(&self) -> bool {
        let (sx, sy) = self.get_scale();
        sx.fuzzy_eq(&sy)
    }

    /// Returns `true` if the transform contains skew part.
    pub fn has_skew(&self) -> bool {
        let (skew_x, skew_y) = self.get_skew();
        skew_x.fuzzy_ne(&0.0) || skew_y.fuzzy_ne(&0.0)
    }

    /// Returns `true` if the transform contains rotate part.
    pub fn has_rotate(&self) -> bool {
        self.get_rotate().fuzzy_ne(&0.0)
    }

    /// Returns transform's translate part.
    #[inline]
    pub fn get_translate(&self) -> (f64, f64) {
        (self.e, self.f)
    }

    /// Returns transform's scale part.
    #[inline]
    pub fn get_scale(&self) -> (f64, f64) {
        let x_scale = (self.a * self.a + self.c * self.c).sqrt();
        let y_scale = (self.b * self.b + self.d * self.d).sqrt();
        (x_scale, y_scale)
    }

    /// Returns transform's skew part.
    #[inline]
    pub fn get_skew(&self) -> (f64, f64) {
        let skew_x = (self.d).atan2(self.c).to_degrees() - 90.0;
        let skew_y = (self.b).atan2(self.a).to_degrees();
        (skew_x, skew_y)
    }

    /// Returns transform's rotate part.
    #[inline]
    pub fn get_rotate(&self) -> f64 {
        let mut angle = (-self.b/self.a).atan().to_degrees();
        if self.b < self.c || self.b > self.c {
            angle = -angle;
        }
        angle
    }

    /// Applies transform to selected coordinates.
    #[inline]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let new_x = self.a * x + self.c * y + self.e;
        let new_y = self.b * x + self.d * y + self.f;
        (new_x, new_y)
    }

    /// Applies transform to selected coordinates.
    #[inline]
    pub fn apply_to(&self, x: &mut f64, y: &mut f64) {
        let tx = *x;
        let ty = *y;
        *x = self.a * tx + self.c * ty + self.e;
        *y = self.b * tx + self.d * ty + self.f;
    }
}

#[inline(never)]
fn multiply(ts1: &Transform, ts2: &Transform) -> Transform {
    Transform {
        a: ts1.a * ts2.a + ts1.c * ts2.b,
        b: ts1.b * ts2.a + ts1.d * ts2.b,
        c: ts1.a * ts2.c + ts1.c * ts2.d,
        d: ts1.b * ts2.c + ts1.d * ts2.d,
        e: ts1.a * ts2.e + ts1.c * ts2.f + ts1.e,
        f: ts1.b * ts2.e + ts1.d * ts2.f + ts1.f,
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Transform {
        Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

impl FuzzyEq for Transform {
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.a.fuzzy_eq(&other.a)
        && self.b.fuzzy_eq(&other.b)
        && self.c.fuzzy_eq(&other.c)
        && self.d.fuzzy_eq(&other.d)
        && self.e.fuzzy_eq(&other.e)
        && self.f.fuzzy_eq(&other.f)
    }
}
