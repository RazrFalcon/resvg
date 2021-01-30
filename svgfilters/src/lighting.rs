// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{ImageRef, ImageRefMut, FuzzyEq, FuzzyZero, RGB8, RGBA8, f64_bound};

const FACTOR_1_2: f64 = 1.0 / 2.0;
const FACTOR_1_3: f64 = 1.0 / 3.0;
const FACTOR_1_4: f64 = 1.0 / 4.0;
const FACTOR_2_3: f64 = 2.0 / 3.0;


/// A light source.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum LightSource {
    DistantLight {
        azimuth: f64,
        elevation: f64,
    },
    PointLight {
        x: f64,
        y: f64,
        z: f64,
    },
    SpotLight {
        x: f64,
        y: f64,
        z: f64,
        points_at_x: f64,
        points_at_y: f64,
        points_at_z: f64,
        specular_exponent: f64,
        limiting_cone_angle: Option<f64>,
    },
}


#[derive(Clone, Copy, Debug)]
struct Vector2 {
    x: f64,
    y: f64,
}

impl Vector2 {
    #[inline]
    fn new(x: f64, y: f64) -> Self {
        Vector2 { x, y }
    }

    #[inline]
    fn is_fuzzy_zero(&self) -> bool {
           self.x.is_fuzzy_zero()
        && self.y.is_fuzzy_zero()
    }
}

impl core::ops::Mul<f64> for Vector2 {
    type Output = Self;

    #[inline]
    fn mul(self, c: f64) -> Self::Output {
        Vector2 {
            x: self.x * c,
            y: self.y * c,
        }
    }
}


#[derive(Clone, Copy, Debug)]
struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector3 {
    #[inline]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vector3 { x, y, z }
    }

    #[inline]
    fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    fn length(&self) -> f64 {
        (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
    }

    #[inline]
    fn normalized(&self) -> Option<Self> {
        let length = self.length();
        if !length.is_fuzzy_zero() {
            Some(Vector3 {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length,
            })
        } else {
            None
        }
    }
}

impl core::ops::Add<Vector3> for Vector3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl core::ops::Sub<Vector3> for Vector3 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}


#[derive(Clone, Copy, Debug)]
struct Normal {
    factor: Vector2,
    normal: Vector2,
}

impl Normal {
    #[inline]
    fn new(factor_x: f64, factor_y: f64, nx: i16, ny: i16) -> Self {
        Normal {
            factor: Vector2::new(factor_x, factor_y),
            normal: Vector2::new(-nx as f64, -ny as f64),
        }
    }
}


/// Renders a diffuse lighting.
///
/// - `src` pixels can have any alpha method, since only the alpha channel is used.
/// - `dest` will have an **unpremultiplied alpha**.
///
/// Does nothing when `src` is less than 3x3.
///
/// # Panics
///
/// - When `LightSource::SpotLight::specular_exponent` is negative.
/// - When `src` and `dest` have different sizes.
pub fn diffuse_lighting(
    surface_scale: f64,
    diffuse_constant: f64,
    lighting_color: RGB8,
    light_source: LightSource,
    src: ImageRef,
    dest: ImageRefMut,
) {
    assert!(src.width == dest.width && src.height == dest.height);

    if let LightSource::SpotLight { specular_exponent, .. } = light_source {
        assert!(!specular_exponent.is_sign_negative());
    }

    let light_factor = |normal: Normal, light_vector: Vector3| {
        let k = if normal.normal.is_fuzzy_zero() {
            light_vector.z
        } else {
            let mut n = normal.normal * (surface_scale / 255.0);
            n.x *= normal.factor.x;
            n.y *= normal.factor.y;

            let normal = Vector3::new(n.x, n.y, 1.0);

            normal.dot(&light_vector) / normal.length()
        };

        diffuse_constant * k
    };

    apply(light_source, surface_scale, lighting_color, &light_factor,
          calc_diffuse_alpha, src, dest);
}

/// Renders a specular lighting.
///
/// - `src` pixels can have any alpha method, since only the alpha channel is used.
/// - `dest` will have a **premultiplied alpha**.
///
/// Does nothing when `src` is less than 3x3.
///
/// # Panics
///
/// - When `LightSource::SpotLight::specular_exponent` is negative.
/// - When `src` and `dest` have different sizes.
pub fn specular_lighting(
    surface_scale: f64,
    specular_constant: f64,
    specular_exponent: f64,
    lighting_color: RGB8,
    light_source: LightSource,
    src: ImageRef,
    dest: ImageRefMut,
) {
    assert!(src.width == dest.width && src.height == dest.height);

    if let LightSource::SpotLight { specular_exponent, .. } = light_source {
        assert!(!specular_exponent.is_sign_negative());
    }

    let light_factor = |normal: Normal, light_vector: Vector3| {
        let h = light_vector + Vector3::new(0.0, 0.0, 1.0);
        let h_length = h.length();

        if h_length.is_fuzzy_zero() {
            return 0.0;
        }

        let k = if normal.normal.is_fuzzy_zero() {
            let n_dot_h = h.z / h_length;
            if specular_exponent.fuzzy_eq(&1.0) {
                n_dot_h
            } else {
                n_dot_h.powf(specular_exponent)
            }
        } else {
            let mut n = normal.normal * (surface_scale / 255.0);
            n.x *= normal.factor.x;
            n.y *= normal.factor.y;

            let normal = Vector3::new(n.x, n.y, 1.0);

            let n_dot_h = normal.dot(&h) / normal.length() / h_length;
            if specular_exponent.fuzzy_eq(&1.0) {
                n_dot_h
            } else {
                n_dot_h.powf(specular_exponent)
            }
        };

        specular_constant * k
    };

    apply(light_source, surface_scale, lighting_color, &light_factor,
          calc_specular_alpha, src, dest);
}

fn apply(
    light_source: LightSource,
    surface_scale: f64,
    lighting_color: RGB8,
    light_factor: &dyn Fn(Normal, Vector3) -> f64,
    calc_alpha: fn(u8, u8, u8) -> u8,
    src: ImageRef,
    mut dest: ImageRefMut,
) {
    if src.width < 3 || src.height < 3 {
        return;
    }

    let width = src.width;
    let height = src.height;

    // `feDistantLight` has a fixed vector, so calculate it beforehand.
    let mut light_vector = match light_source {
        LightSource::DistantLight { azimuth, elevation } => {
            let azimuth = azimuth.to_radians();
            let elevation = elevation.to_radians();
            Vector3::new(
                azimuth.cos() * elevation.cos(),
                azimuth.sin() * elevation.cos(),
                elevation.sin(),
            )
        }
        _ => Vector3::new(1.0, 1.0, 1.0),
    };

    let mut calc = |nx, ny, normal: Normal| {
        match light_source {
            LightSource::DistantLight { .. } => {}
            LightSource::PointLight { x, y, z } | LightSource::SpotLight { x, y, z, .. } => {
                let nz = src.alpha_at(nx, ny) as f64 / 255.0 * surface_scale;
                let origin = Vector3::new(x, y, z);
                let v = origin - Vector3::new(nx as f64, ny as f64, nz);
                light_vector = v.normalized().unwrap_or(v);
            }
        }

        let light_color = light_color(&light_source, lighting_color, light_vector);
        let factor = light_factor(normal, light_vector);

        let compute = |x| (f64_bound(0.0, x as f64 * factor, 255.0) + 0.5) as u8;

        let r = compute(light_color.r);
        let g = compute(light_color.g);
        let b = compute(light_color.b);
        let a = calc_alpha(r, g, b);

        *dest.pixel_at_mut(nx, ny) = RGBA8 { b, g, r, a };
    };

    calc(0,         0,          top_left_normal(src));
    calc(width - 1, 0,          top_right_normal(src));
    calc(0,         height - 1, bottom_left_normal(src));
    calc(width - 1, height - 1, bottom_right_normal(src));

    for x in 1..width-1 {
        calc(x, 0,          top_row_normal(src, x));
        calc(x, height - 1, bottom_row_normal(src, x));
    }

    for y in 1..height-1 {
        calc(0,         y, left_column_normal(src, y));
        calc(width - 1, y, right_column_normal(src, y));
    }

    for y in 1..height-1 {
        for x in 1..width-1 {
            calc(x, y, interior_normal(src, x, y));
        }
    }
}

fn light_color(
    light: &LightSource,
    lighting_color: RGB8,
    light_vector: Vector3,
) -> RGB8 {
    match *light {
        LightSource::DistantLight { .. } | LightSource::PointLight { .. } => {
            lighting_color
        }
        LightSource::SpotLight {
            x, y, z, points_at_x, points_at_y, points_at_z, specular_exponent, limiting_cone_angle
        } => {
            let origin = Vector3::new(x, y, z);
            let direction = Vector3::new(points_at_x, points_at_y, points_at_z);
            let direction = direction - origin;
            let direction = direction.normalized().unwrap_or(direction);
            let minus_l_dot_s = -light_vector.dot(&direction);
            if minus_l_dot_s <= 0.0 {
                return RGB8::default();
            }

            if let Some(limiting_cone_angle) = limiting_cone_angle {
                if minus_l_dot_s < limiting_cone_angle.to_radians().cos() {
                    return RGB8::default();
                }
            }

            let factor = minus_l_dot_s.powf(specular_exponent);
            let compute = |x| (f64_bound(0.0, x as f64 * factor, 255.0) + 0.5) as u8;

            RGB8 {
                r: compute(lighting_color.r),
                g: compute(lighting_color.g),
                b: compute(lighting_color.b),
            }
        }
    }
}

fn top_left_normal(img: ImageRef) -> Normal {
    let center       = img.alpha_at(0, 0);
    let right        = img.alpha_at(1, 0);
    let bottom       = img.alpha_at(0, 1);
    let bottom_right = img.alpha_at(1, 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -2 * center + 2 * right - bottom + bottom_right,
        -2 * center - right + 2 * bottom + bottom_right,
    )
}

fn top_right_normal(img: ImageRef) -> Normal {
    let left         = img.alpha_at(img.width - 2, 0);
    let center       = img.alpha_at(img.width - 1, 0);
    let bottom_left  = img.alpha_at(img.width - 2, 1);
    let bottom       = img.alpha_at(img.width - 1, 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -2 * left + 2 * center - bottom_left + bottom,
        -left - 2 * center + bottom_left + 2 * bottom,
    )
}

fn bottom_left_normal(img: ImageRef) -> Normal {
    let top          = img.alpha_at(0, img.height - 2);
    let top_right    = img.alpha_at(1, img.height - 2);
    let center       = img.alpha_at(0, img.height - 1);
    let right        = img.alpha_at(1, img.height - 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -top + top_right - 2 * center + 2 * right,
        -2 * top - top_right + 2 * center + right,
    )
}

fn bottom_right_normal(img: ImageRef) -> Normal {
    let top_left     = img.alpha_at(img.width - 2, img.height - 2);
    let top          = img.alpha_at(img.width - 1, img.height - 2);
    let left         = img.alpha_at(img.width - 2, img.height - 1);
    let center       = img.alpha_at(img.width - 1, img.height - 1);

    Normal::new(
        FACTOR_2_3,
        FACTOR_2_3,
        -top_left + top - 2 * left + 2 * center,
        -top_left - 2 * top + left + 2 * center,
    )
}

fn top_row_normal(img: ImageRef, x: u32) -> Normal {
    let left         = img.alpha_at(x - 1, 0);
    let center       = img.alpha_at(x,     0);
    let right        = img.alpha_at(x + 1, 0);
    let bottom_left  = img.alpha_at(x - 1, 1);
    let bottom       = img.alpha_at(x,     1);
    let bottom_right = img.alpha_at(x + 1, 1);

    Normal::new(
        FACTOR_1_3,
        FACTOR_1_2,
        -2 * left + 2 * right - bottom_left + bottom_right,
        -left - 2 * center - right + bottom_left + 2 * bottom + bottom_right,
    )
}

fn bottom_row_normal(img: ImageRef, x: u32) -> Normal {
    let top_left     = img.alpha_at(x - 1, img.height - 2);
    let top          = img.alpha_at(x,     img.height - 2);
    let top_right    = img.alpha_at(x + 1, img.height - 2);
    let left         = img.alpha_at(x - 1, img.height - 1);
    let center       = img.alpha_at(x,     img.height - 1);
    let right        = img.alpha_at(x + 1, img.height - 1);

    Normal::new(
        FACTOR_1_3,
        FACTOR_1_2,
        -top_left + top_right - 2 * left + 2 * right,
        -top_left - 2 * top - top_right + left + 2 * center + right,
    )
}

fn left_column_normal(img: ImageRef, y: u32) -> Normal {
    let top          = img.alpha_at(0, y - 1);
    let top_right    = img.alpha_at(1, y - 1);
    let center       = img.alpha_at(0, y);
    let right        = img.alpha_at(1, y);
    let bottom       = img.alpha_at(0, y + 1);
    let bottom_right = img.alpha_at(1, y + 1);

    Normal::new(
        FACTOR_1_2,
        FACTOR_1_3,
        -top + top_right - 2 * center + 2 * right - bottom + bottom_right,
        -2 * top - top_right + 2 * bottom + bottom_right,
    )
}

fn right_column_normal(img: ImageRef, y: u32) -> Normal {
    let top_left     = img.alpha_at(img.width - 2, y - 1);
    let top          = img.alpha_at(img.width - 1, y - 1);
    let left         = img.alpha_at(img.width - 2, y);
    let center       = img.alpha_at(img.width - 1, y);
    let bottom_left  = img.alpha_at(img.width - 2, y + 1);
    let bottom       = img.alpha_at(img.width - 1, y + 1);

    Normal::new(
        FACTOR_1_2,
        FACTOR_1_3,
        -top_left + top - 2 * left + 2 * center - bottom_left + bottom,
        -top_left - 2 * top + bottom_left + 2 * bottom,
    )
}

fn interior_normal(img: ImageRef, x: u32, y: u32) -> Normal {
    let top_left     = img.alpha_at(x - 1, y - 1);
    let top          = img.alpha_at(x,     y - 1);
    let top_right    = img.alpha_at(x + 1, y - 1);
    let left         = img.alpha_at(x - 1, y);
    let right        = img.alpha_at(x + 1, y);
    let bottom_left  = img.alpha_at(x - 1, y + 1);
    let bottom       = img.alpha_at(x,     y + 1);
    let bottom_right = img.alpha_at(x + 1, y + 1);

    Normal::new(
        FACTOR_1_4,
        FACTOR_1_4,
        -top_left + top_right - 2 * left + 2 * right - bottom_left + bottom_right,
        -top_left - 2 * top - top_right + bottom_left + 2 * bottom + bottom_right,
    )
}

fn calc_diffuse_alpha(_: u8, _: u8, _: u8) -> u8 {
    255
}

fn calc_specular_alpha(r: u8, g: u8, b: u8) -> u8 {
    use core::cmp::max;
    max(max(r, g), b)
}
