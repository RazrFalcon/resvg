// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! SVG filter types.

use std::cell::RefCell;
use std::rc::Rc;

use strict_num::PositiveF32;
use svgtypes::AspectRatio;

use crate::{BlendMode, Color, Group, ImageRendering, NonZeroF32, NonZeroRect, Opacity, Units};

/// A filter element.
///
/// `filter` element in the SVG.
#[derive(Clone, Debug)]
pub struct Filter {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub id: String,

    /// Region coordinate system units.
    ///
    /// `filterUnits` in the SVG.
    pub units: Units,

    /// Content coordinate system units.
    ///
    /// `primitiveUnits` in the SVG.
    pub primitive_units: Units,

    /// Filter region.
    ///
    /// `x`, `y`, `width` and `height` in the SVG.
    pub rect: NonZeroRect,

    /// A list of filter primitives.
    pub primitives: Vec<Primitive>,
}

/// An alias for a shared `Filter`.
pub type SharedFilter = Rc<RefCell<Filter>>;

/// A filter primitive element.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// `x` coordinate of the filter subregion.
    pub x: Option<f32>,

    /// `y` coordinate of the filter subregion.
    pub y: Option<f32>,

    /// The filter subregion width.
    pub width: Option<f32>,

    /// The filter subregion height.
    pub height: Option<f32>,

    /// Color interpolation mode.
    ///
    /// `color-interpolation-filters` in the SVG.
    pub color_interpolation: ColorInterpolation,

    /// Assigned name for this filter primitive.
    ///
    /// `result` in the SVG.
    pub result: String,

    /// Filter primitive kind.
    pub kind: Kind,
}

/// A filter kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum Kind {
    Blend(Blend),
    ColorMatrix(ColorMatrix),
    ComponentTransfer(ComponentTransfer),
    Composite(Composite),
    ConvolveMatrix(ConvolveMatrix),
    DiffuseLighting(DiffuseLighting),
    DisplacementMap(DisplacementMap),
    DropShadow(DropShadow),
    Flood(Flood),
    GaussianBlur(GaussianBlur),
    Image(Image),
    Merge(Merge),
    Morphology(Morphology),
    Offset(Offset),
    SpecularLighting(SpecularLighting),
    Tile(Tile),
    Turbulence(Turbulence),
}

impl Kind {
    /// Checks that `FilterKind` has a specific input.
    pub fn has_input(&self, input: &Input) -> bool {
        match self {
            Kind::Blend(ref fe) => fe.input1 == *input || fe.input2 == *input,
            Kind::ColorMatrix(ref fe) => fe.input == *input,
            Kind::ComponentTransfer(ref fe) => fe.input == *input,
            Kind::Composite(ref fe) => fe.input1 == *input || fe.input2 == *input,
            Kind::ConvolveMatrix(ref fe) => fe.input == *input,
            Kind::DiffuseLighting(ref fe) => fe.input == *input,
            Kind::DisplacementMap(ref fe) => fe.input1 == *input || fe.input2 == *input,
            Kind::DropShadow(ref fe) => fe.input == *input,
            Kind::Flood(_) => false,
            Kind::GaussianBlur(ref fe) => fe.input == *input,
            Kind::Image(_) => false,
            Kind::Merge(ref fe) => fe.inputs.iter().any(|i| i == input),
            Kind::Morphology(ref fe) => fe.input == *input,
            Kind::Offset(ref fe) => fe.input == *input,
            Kind::SpecularLighting(ref fe) => fe.input == *input,
            Kind::Tile(ref fe) => fe.input == *input,
            Kind::Turbulence(_) => false,
        }
    }
}

/// Identifies input for a filter primitive.
#[allow(missing_docs)]
#[derive(Clone, PartialEq, Debug)]
pub enum Input {
    SourceGraphic,
    SourceAlpha,
    Reference(String),
}

/// A color interpolation mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorInterpolation {
    SRGB,
    LinearRGB,
}

impl Default for ColorInterpolation {
    fn default() -> Self {
        ColorInterpolation::LinearRGB
    }
}

/// A blend filter primitive.
///
/// `feBlend` element in the SVG.
#[derive(Clone, Debug)]
pub struct Blend {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// A blending mode.
    ///
    /// `mode` in the SVG.
    pub mode: BlendMode,
}

/// A color matrix filter primitive.
///
/// `feColorMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct ColorMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A matrix kind.
    ///
    /// `type` in the SVG.
    pub kind: ColorMatrixKind,
}

/// A color matrix filter primitive kind.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum ColorMatrixKind {
    Matrix(Vec<f32>), // Guarantee to have 20 numbers.
    Saturate(PositiveF32),
    HueRotate(f32),
    LuminanceToAlpha,
}

impl Default for ColorMatrixKind {
    fn default() -> Self {
        ColorMatrixKind::Matrix(vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ])
    }
}

/// A component-wise remapping filter primitive.
///
/// `feComponentTransfer` element in the SVG.
#[derive(Clone, Debug)]
pub struct ComponentTransfer {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// `feFuncR` in the SVG.
    pub func_r: TransferFunction,

    /// `feFuncG` in the SVG.
    pub func_g: TransferFunction,

    /// `feFuncB` in the SVG.
    pub func_b: TransferFunction,

    /// `feFuncA` in the SVG.
    pub func_a: TransferFunction,
}

/// A transfer function used by `FeComponentTransfer`.
///
/// <https://www.w3.org/TR/SVG11/filters.html#transferFuncElements>
#[derive(Clone, Debug)]
pub enum TransferFunction {
    /// Keeps a component as is.
    Identity,

    /// Applies a linear interpolation to a component.
    ///
    /// The number list can be empty.
    Table(Vec<f32>),

    /// Applies a step function to a component.
    ///
    /// The number list can be empty.
    Discrete(Vec<f32>),

    /// Applies a linear shift to a component.
    #[allow(missing_docs)]
    Linear { slope: f32, intercept: f32 },

    /// Applies an exponential shift to a component.
    #[allow(missing_docs)]
    Gamma {
        amplitude: f32,
        exponent: f32,
        offset: f32,
    },
}

/// A composite filter primitive.
///
/// `feComposite` element in the SVG.
#[derive(Clone, Debug)]
pub struct Composite {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// A compositing operation.
    ///
    /// `operator` in the SVG.
    pub operator: CompositeOperator,
}

/// An images compositing operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CompositeOperator {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Arithmetic { k1: f32, k2: f32, k3: f32, k4: f32 },
}

/// A matrix convolution filter primitive.
///
/// `feConvolveMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct ConvolveMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A convolve matrix.
    pub matrix: ConvolveMatrixData,

    /// A matrix divisor.
    ///
    /// `divisor` in the SVG.
    pub divisor: NonZeroF32,

    /// A kernel matrix bias.
    ///
    /// `bias` in the SVG.
    pub bias: f32,

    /// An edges processing mode.
    ///
    /// `edgeMode` in the SVG.
    pub edge_mode: EdgeMode,

    /// An alpha preserving flag.
    ///
    /// `preserveAlpha` in the SVG.
    pub preserve_alpha: bool,
}

/// A convolve matrix representation.
///
/// Used primarily by [`ConvolveMatrix`].
#[derive(Clone, Debug)]
pub struct ConvolveMatrixData {
    /// Returns a matrix's X target.
    ///
    /// `targetX` in the SVG.
    pub target_x: u32,

    /// Returns a matrix's Y target.
    ///
    /// `targetY` in the SVG.
    pub target_y: u32,

    /// Returns a number of columns in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    pub columns: u32,

    /// Returns a number of rows in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    pub rows: u32,

    /// The actual matrix.
    pub data: Vec<f32>,
}

impl ConvolveMatrixData {
    /// Creates a new `ConvolveMatrixData`.
    ///
    /// Returns `None` when:
    ///
    /// - `columns` * `rows` != `data.len()`
    /// - `target_x` >= `columns`
    /// - `target_y` >= `rows`
    pub fn new(
        target_x: u32,
        target_y: u32,
        columns: u32,
        rows: u32,
        data: Vec<f32>,
    ) -> Option<Self> {
        if (columns * rows) as usize != data.len() || target_x >= columns || target_y >= rows {
            return None;
        }

        Some(ConvolveMatrixData {
            target_x,
            target_y,
            columns,
            rows,
            data,
        })
    }

    /// Returns a matrix value at the specified position.
    ///
    /// # Panics
    ///
    /// - When position is out of bounds.
    pub fn get(&self, x: u32, y: u32) -> f32 {
        self.data[(y * self.columns + x) as usize]
    }
}

/// An edges processing mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EdgeMode {
    None,
    Duplicate,
    Wrap,
}

/// A displacement map filter primitive.
///
/// `feDisplacementMap` element in the SVG.
#[derive(Clone, Debug)]
pub struct DisplacementMap {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// Scale factor.
    ///
    /// `scale` in the SVG.
    pub scale: f32,

    /// Indicates a source color channel along the X-axis.
    ///
    /// `xChannelSelector` in the SVG.
    pub x_channel_selector: ColorChannel,

    /// Indicates a source color channel along the Y-axis.
    ///
    /// `yChannelSelector` in the SVG.
    pub y_channel_selector: ColorChannel,
}

/// A color channel.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorChannel {
    R,
    G,
    B,
    A,
}

/// A drop shadow filter primitive.
///
/// This is essentially `feGaussianBlur`, `feOffset` and `feFlood` joined together.
///
/// `feDropShadow` element in the SVG.
#[derive(Clone, Debug)]
pub struct DropShadow {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// The amount to offset the input graphic along the X-axis.
    pub dx: f32,

    /// The amount to offset the input graphic along the Y-axis.
    pub dy: f32,

    /// A standard deviation along the X-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_x: PositiveF32,

    /// A standard deviation along the Y-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_y: PositiveF32,

    /// A flood color.
    ///
    /// `flood-color` in the SVG.
    pub color: Color,

    /// A flood opacity.
    ///
    /// `flood-opacity` in the SVG.
    pub opacity: Opacity,
}

/// A flood filter primitive.
///
/// `feFlood` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct Flood {
    /// A flood color.
    ///
    /// `flood-color` in the SVG.
    pub color: Color,

    /// A flood opacity.
    ///
    /// `flood-opacity` in the SVG.
    pub opacity: Opacity,
}

/// A Gaussian blur filter primitive.
///
/// `feGaussianBlur` element in the SVG.
#[derive(Clone, Debug)]
pub struct GaussianBlur {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A standard deviation along the X-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_x: PositiveF32,

    /// A standard deviation along the Y-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_y: PositiveF32,
}

/// An image filter primitive.
///
/// `feImage` element in the SVG.
#[derive(Clone, Debug)]
pub struct Image {
    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,

    /// Rendering method.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub data: ImageKind,
}

/// Kind of the `feImage` data.
#[derive(Clone, Debug)]
pub enum ImageKind {
    /// An image data.
    Image(crate::ImageKind),

    /// An SVG node.
    Use(Box<Group>),
}

/// A diffuse lighting filter primitive.
///
/// `feDiffuseLighting` element in the SVG.
#[derive(Clone, Debug)]
pub struct DiffuseLighting {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A surface scale.
    ///
    /// `surfaceScale` in the SVG.
    pub surface_scale: f32,

    /// A diffuse constant.
    ///
    /// `diffuseConstant` in the SVG.
    pub diffuse_constant: f32,

    /// A lighting color.
    ///
    /// `lighting-color` in the SVG.
    pub lighting_color: Color,

    /// A light source.
    pub light_source: LightSource,
}

/// A specular lighting filter primitive.
///
/// `feSpecularLighting` element in the SVG.
#[derive(Clone, Debug)]
pub struct SpecularLighting {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A surface scale.
    ///
    /// `surfaceScale` in the SVG.
    pub surface_scale: f32,

    /// A specular constant.
    ///
    /// `specularConstant` in the SVG.
    pub specular_constant: f32,

    /// A specular exponent.
    ///
    /// Should be in 1..128 range.
    ///
    /// `specularExponent` in the SVG.
    pub specular_exponent: f32,

    /// A lighting color.
    ///
    /// `lighting-color` in the SVG.
    pub lighting_color: Color,

    /// A light source.
    pub light_source: LightSource,
}

/// A light source kind.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum LightSource {
    DistantLight(DistantLight),
    PointLight(PointLight),
    SpotLight(SpotLight),
}

/// A distant light source.
///
/// `feDistantLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct DistantLight {
    /// Direction angle for the light source on the XY plane (clockwise),
    /// in degrees from the x axis.
    ///
    /// `azimuth` in the SVG.
    pub azimuth: f32,

    /// Direction angle for the light source from the XY plane towards the z axis, in degrees.
    ///
    /// `elevation` in the SVG.
    pub elevation: f32,
}

/// A point light source.
///
/// `fePointLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    /// X location for the light source.
    ///
    /// `x` in the SVG.
    pub x: f32,

    /// Y location for the light source.
    ///
    /// `y` in the SVG.
    pub y: f32,

    /// Z location for the light source.
    ///
    /// `z` in the SVG.
    pub z: f32,
}

/// A spot light source.
///
/// `feSpotLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct SpotLight {
    /// X location for the light source.
    ///
    /// `x` in the SVG.
    pub x: f32,

    /// Y location for the light source.
    ///
    /// `y` in the SVG.
    pub y: f32,

    /// Z location for the light source.
    ///
    /// `z` in the SVG.
    pub z: f32,

    /// X point at which the light source is pointing.
    ///
    /// `pointsAtX` in the SVG.
    pub points_at_x: f32,

    /// Y point at which the light source is pointing.
    ///
    /// `pointsAtY` in the SVG.
    pub points_at_y: f32,

    /// Z point at which the light source is pointing.
    ///
    /// `pointsAtZ` in the SVG.
    pub points_at_z: f32,

    /// Exponent value controlling the focus for the light source.
    ///
    /// `specularExponent` in the SVG.
    pub specular_exponent: PositiveF32,

    /// A limiting cone which restricts the region where the light is projected.
    ///
    /// `limitingConeAngle` in the SVG.
    pub limiting_cone_angle: Option<f32>,
}

/// A merge filter primitive.
///
/// `feMerge` element in the SVG.
#[derive(Clone, Debug)]
pub struct Merge {
    /// List of input layers that should be merged.
    ///
    /// List of `feMergeNode`'s in the SVG.
    pub inputs: Vec<Input>,
}

/// A morphology filter primitive.
///
/// `feMorphology` element in the SVG.
#[derive(Clone, Debug)]
pub struct Morphology {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A filter operator.
    ///
    /// `operator` in the SVG.
    pub operator: MorphologyOperator,

    /// A filter radius along the X-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_x: PositiveF32,

    /// A filter radius along the Y-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_y: PositiveF32,
}

/// A morphology operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MorphologyOperator {
    Erode,
    Dilate,
}

/// An offset filter primitive.
///
/// `feOffset` element in the SVG.
#[derive(Clone, Debug)]
pub struct Offset {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// The amount to offset the input graphic along the X-axis.
    pub dx: f32,

    /// The amount to offset the input graphic along the Y-axis.
    pub dy: f32,
}

/// A tile filter primitive.
///
/// `feTile` element in the SVG.
#[derive(Clone, Debug)]
pub struct Tile {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,
}

/// A turbulence generation filter primitive.
///
/// `feTurbulence` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct Turbulence {
    /// Identifies the base frequency for the noise function.
    ///
    /// `baseFrequency` in the SVG.
    pub base_frequency_x: PositiveF32,

    /// Identifies the base frequency for the noise function.
    ///
    /// `baseFrequency` in the SVG.
    pub base_frequency_y: PositiveF32,

    /// Identifies the number of octaves for the noise function.
    ///
    /// `numOctaves` in the SVG.
    pub num_octaves: u32,

    /// The starting number for the pseudo random number generator.
    ///
    /// `seed` in the SVG.
    pub seed: i32,

    /// Smooth transitions at the border of tiles.
    ///
    /// `stitchTiles` in the SVG.
    pub stitch_tiles: bool,

    /// Indicates whether the filter primitive should perform a noise or turbulence function.
    ///
    /// `type` in the SVG.
    pub kind: TurbulenceKind,
}

/// A turbulence kind for the `feTurbulence` filter.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TurbulenceKind {
    FractalNoise,
    Turbulence,
}
