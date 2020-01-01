// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ops::Deref;
use std::rc::Rc;

use crate::geom::*;
use super::attributes::*;
use super::pathdata::PathData;

// TODO: implement Default for all


/// Node's kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum NodeKind {
    Svg(Svg),
    Defs,
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    ClipPath(ClipPath),
    Mask(Mask),
    Pattern(Pattern),
    Filter(Filter),
    Path(Path),
    Image(Image),
    Group(Group),
}

impl NodeKind {
    /// Returns node's ID.
    ///
    /// If a current node doesn't support ID - an empty string
    /// will be returned.
    pub fn id(&self) -> &str {
        match *self {
            NodeKind::Svg(_) => "",
            NodeKind::Defs => "",
            NodeKind::LinearGradient(ref e) => e.id.as_str(),
            NodeKind::RadialGradient(ref e) => e.id.as_str(),
            NodeKind::ClipPath(ref e) => e.id.as_str(),
            NodeKind::Mask(ref e) => e.id.as_str(),
            NodeKind::Pattern(ref e) => e.id.as_str(),
            NodeKind::Filter(ref e) => e.id.as_str(),
            NodeKind::Path(ref e) => e.id.as_str(),
            NodeKind::Image(ref e) => e.id.as_str(),
            NodeKind::Group(ref e) => e.id.as_str(),
        }
    }

    /// Returns node's transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    pub fn transform(&self) -> Transform {
        match *self {
            NodeKind::Svg(_) => Transform::default(),
            NodeKind::Defs => Transform::default(),
            NodeKind::LinearGradient(ref e) => e.transform,
            NodeKind::RadialGradient(ref e) => e.transform,
            NodeKind::ClipPath(ref e) => e.transform,
            NodeKind::Mask(_) => Transform::default(),
            NodeKind::Pattern(ref e) => e.transform,
            NodeKind::Filter(_) => Transform::default(),
            NodeKind::Path(ref e) => e.transform,
            NodeKind::Image(ref e) => e.transform,
            NodeKind::Group(ref e) => e.transform,
        }
    }
}


/// An SVG root element.
#[derive(Clone, Copy, Debug)]
pub struct Svg {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    ///
    /// `width` and `height` in SVG.
    pub size: Size,

    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    ///
    /// `viewBox` and `preserveAspectRatio` in SVG.
    pub view_box: ViewBox,
}


/// A path element.
#[derive(Clone, Debug)]
pub struct Path {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Element visibility.
    pub visibility: Visibility,

    /// Fill style.
    pub fill: Option<Fill>,

    /// Stroke style.
    pub stroke: Option<Stroke>,

    /// Rendering mode.
    ///
    /// `shape-rendering` in SVG.
    pub rendering_mode: ShapeRendering,

    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub data: Rc<PathData>,
}

impl Default for Path {
    fn default() -> Self {
        Path {
            id: String::new(),
            transform: Transform::default(),
            visibility: Visibility::Visible,
            fill: None,
            stroke: None,
            rendering_mode: ShapeRendering::default(),
            data: Rc::new(PathData::default()),
        }
    }
}


/// A raster image element.
///
/// `image` element in SVG.
#[derive(Clone, Debug)]
pub struct Image {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Element visibility.
    pub visibility: Visibility,

    /// An image rectangle in which it should be fit.
    ///
    /// Combination of the `x`, `y`, `width`, `height` and `preserveAspectRatio`
    /// attributes.
    pub view_box: ViewBox,

    /// Rendering mode.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub data: ImageData,

    /// Image data kind.
    pub format: ImageFormat,
}


/// A group container.
///
/// The preprocessor will remove all groups that don't impact rendering.
/// Those that left is just an indicator that a new canvas should be created.
///
/// `g` element in SVG.
#[derive(Clone, Debug)]
pub struct Group {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Group opacity.
    ///
    /// After the group is rendered we should combine
    /// it with a parent group using the specified opacity.
    pub opacity: Opacity,

    /// Element clip path.
    pub clip_path: Option<String>,

    /// Element mask.
    pub mask: Option<String>,

    /// Element filter.
    pub filter: Option<String>,

    /// Contains a fill color or paint server used by `FilterInput::FillPaint`.
    ///
    /// Will be set only when filter actually has a `FilterInput::FillPaint`.
    pub filter_fill: Option<Paint>,

    /// Contains a fill color or paint server used by `FilterInput::StrokePaint`.
    ///
    /// Will be set only when filter actually has a `FilterInput::StrokePaint`.
    pub filter_stroke: Option<Paint>,

    /// Indicates that this node can be accessed via `filter`.
    ///
    /// `None` indicates an `accumulate` value.
    pub enable_background: Option<EnableBackground>,
}

impl Default for Group {
    fn default() -> Self {
        Group {
            id: String::new(),
            transform: Transform::default(),
            opacity: Opacity::default(),
            clip_path: None,
            mask: None,
            filter: None,
            filter_fill: None,
            filter_stroke: None,
            enable_background: None,
        }
    }
}


/// A generic gradient.
#[derive(Clone, Debug)]
pub struct BaseGradient {
    /// Coordinate system units.
    ///
    /// `gradientUnits` in SVG.
    pub units: Units,

    /// Gradient transform.
    ///
    /// `gradientTransform` in SVG.
    pub transform: Transform,

    /// Gradient spreading method.
    ///
    /// `spreadMethod` in SVG.
    pub spread_method: SpreadMethod,

    /// A list of `stop` elements.
    pub stops: Vec<Stop>,
}


/// A linear gradient.
///
/// `linearGradient` element in SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct LinearGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,

    /// Base gradient data.
    pub base: BaseGradient,
}

impl Deref for LinearGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}


/// A radial gradient.
///
/// `radialGradient` element in SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct RadialGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    pub cx: f64,
    pub cy: f64,
    pub r: PositiveNumber,
    pub fx: f64,
    pub fy: f64,

    /// Base gradient data.
    pub base: BaseGradient,
}

impl Deref for RadialGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}


/// Gradient's stop element.
///
/// `stop` element in SVG.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    /// Gradient stop offset.
    ///
    /// `offset` in SVG.
    pub offset: StopOffset,

    /// Gradient stop color.
    ///
    /// `stop-color` in SVG.
    pub color: Color,

    /// Gradient stop opacity.
    ///
    /// `stop-opacity` in SVG.
    pub opacity: Opacity,
}


/// A clip-path element.
///
/// `clipPath` element in SVG.
#[derive(Clone, Debug)]
pub struct ClipPath {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `clipPathUnits` in SVG.
    pub units: Units,

    /// Clip path transform.
    ///
    /// `transform` in SVG.
    pub transform: Transform,

    /// Additional clip path.
    ///
    /// `clip-path` in SVG.
    pub clip_path: Option<String>,
}

impl Default for ClipPath {
    fn default() -> Self {
        ClipPath {
            id: String::new(),
            units: Units::UserSpaceOnUse,
            transform: Transform::default(),
            clip_path: None,
        }
    }
}


/// A mask element.
///
/// `mask` element in SVG.
#[derive(Clone, Debug)]
pub struct Mask {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `maskUnits` in SVG.
    pub units: Units,

    /// Content coordinate system units.
    ///
    /// `maskContentUnits` in SVG.
    pub content_units: Units,

    /// Mask rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Additional mask.
    ///
    /// `mask` in SVG.
    pub mask: Option<String>,
}


/// A pattern element.
///
/// `pattern` element in SVG.
#[derive(Clone, Debug)]
pub struct Pattern {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `patternUnits` in SVG.
    pub units: Units,

    // TODO: should not be accessible when `viewBox` is present.
    /// Content coordinate system units.
    ///
    /// `patternContentUnits` in SVG.
    pub content_units: Units,

    /// Pattern transform.
    ///
    /// `patternTransform` in SVG.
    pub transform: Transform,

    /// Pattern rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Pattern viewbox.
    pub view_box: Option<ViewBox>,
}


/// A filter element.
///
/// `filter` element in the SVG.
#[derive(Clone, Debug)]
pub struct Filter {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
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
    pub rect: Rect,

    /// A list of filter primitives.
    pub children: Vec<FilterPrimitive>,
}


/// A filter primitive element.
#[derive(Clone, Debug)]
pub struct FilterPrimitive {
    /// `x` coordinate of the filter subregion.
    pub x: Option<f64>,

    /// `y` coordinate of the filter subregion.
    pub y: Option<f64>,

    /// The filter subregion width.
    pub width: Option<f64>,

    /// The filter subregion height.
    pub height: Option<f64>,

    /// Color interpolation mode.
    ///
    /// `color-interpolation-filters` in the SVG.
    pub color_interpolation: ColorInterpolation,

    /// Assigned name for this filter primitive.
    ///
    /// `result` in the SVG.
    pub result: String,

    /// Filter primitive kind.
    pub kind: FilterKind,
}


/// A filter kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum FilterKind {
    FeBlend(FeBlend),
    FeColorMatrix(FeColorMatrix),
    FeComponentTransfer(FeComponentTransfer),
    FeComposite(FeComposite),
    FeConvolveMatrix(FeConvolveMatrix),
    FeDiffuseLighting(FeDiffuseLighting),
    FeDisplacementMap(FeDisplacementMap),
    FeFlood(FeFlood),
    FeGaussianBlur(FeGaussianBlur),
    FeImage(FeImage),
    FeMerge(FeMerge),
    FeMorphology(FeMorphology),
    FeOffset(FeOffset),
    FeSpecularLighting(FeSpecularLighting),
    FeTile(FeTile),
    FeTurbulence(FeTurbulence),
}

impl FilterKind {
    /// Checks that `FilterKind` has a specific input.
    pub fn has_input(&self, input: &FilterInput) -> bool {
        match self {
            FilterKind::FeBlend(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeColorMatrix(ref fe) => fe.input == *input,
            FilterKind::FeComponentTransfer(ref fe) => fe.input == *input,
            FilterKind::FeComposite(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeConvolveMatrix(ref fe) => fe.input == *input,
            FilterKind::FeDiffuseLighting(ref fe) => fe.input == *input,
            FilterKind::FeDisplacementMap(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeFlood(_) => false,
            FilterKind::FeGaussianBlur(ref fe) => fe.input == *input,
            FilterKind::FeImage(_) => false,
            FilterKind::FeMerge(ref fe) => fe.inputs.iter().any(|i| i == input),
            FilterKind::FeMorphology(ref fe) => fe.input == *input,
            FilterKind::FeOffset(ref fe) => fe.input == *input,
            FilterKind::FeSpecularLighting(ref fe) => fe.input == *input,
            FilterKind::FeTile(ref fe) => fe.input == *input,
            FilterKind::FeTurbulence(_) => false,
        }
    }
}


/// A blend filter primitive.
///
/// `feBlend` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeBlend {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,

    /// A blending mode.
    ///
    /// `mode` in the SVG.
    pub mode: FeBlendMode,
}


/// A color matrix filter primitive.
///
/// `feColorMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeColorMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A matrix kind.
    ///
    /// `type` in the SVG.
    pub kind: FeColorMatrixKind,
}

/// A color matrix filter primitive kind.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum FeColorMatrixKind {
    Matrix(Vec<f64>), // Guarantee to have 20 numbers.
    Saturate(NormalizedValue),
    HueRotate(f64),
    LuminanceToAlpha,
}

impl Default for FeColorMatrixKind {
    fn default() -> Self {
        FeColorMatrixKind::Matrix(vec![
            1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }
}


/// A component-wise remapping filter primitive.
///
/// `feComponentTransfer` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeComponentTransfer {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

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
/// https://www.w3.org/TR/SVG11/filters.html#transferFuncElements
#[derive(Clone, Debug)]
pub enum TransferFunction {
    /// Keeps a component as is.
    Identity,

    /// Applies a linear interpolation to a component.
    ///
    /// The number list can be empty.
    Table(Vec<f64>),

    /// Applies a step function to a component.
    ///
    /// The number list can be empty.
    Discrete(Vec<f64>),

    /// Applies a linear shift to a component.
    #[allow(missing_docs)]
    Linear {
        slope: f64,
        intercept: f64,
    },

    /// Applies an exponential shift to a component.
    #[allow(missing_docs)]
    Gamma {
        amplitude: f64,
        exponent: f64,
        offset: f64,
    },
}


/// A composite filter primitive.
///
/// `feComposite` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeComposite {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,

    /// A compositing operation.
    ///
    /// `operator` in the SVG.
    pub operator: FeCompositeOperator,
}


/// A matrix convolution filter primitive.
///
/// `feConvolveMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeConvolveMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A convolve matrix.
    pub matrix: ConvolveMatrix,

    /// A matrix divisor.
    ///
    /// `divisor` in the SVG.
    pub divisor: NonZeroF64,

    /// A kernel matrix bias.
    ///
    /// `bias` in the SVG.
    pub bias: f64,

    /// An edges processing mode.
    ///
    /// `edgeMode` in the SVG.
    pub edge_mode: FeEdgeMode,

    /// An alpha preserving flag.
    ///
    /// `preserveAlpha` in the SVG.
    pub preserve_alpha: bool,
}


/// A diffuse lighting filter primitive.
///
/// `feDiffuseLighting` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeDiffuseLighting {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A surface scale.
    ///
    /// `surfaceScale` in the SVG.
    pub surface_scale: f64,

    /// A diffuse constant.
    ///
    /// `diffuseConstant` in the SVG.
    pub diffuse_constant: f64,

    /// A lighting color.
    ///
    /// `lighting-color` in the SVG.
    pub lighting_color: Color,

    /// A light source.
    pub light_source: FeLightSource,
}


/// A displacement map filter primitive.
///
/// `feDisplacementMap` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeDisplacementMap {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,

    /// Scale factor.
    ///
    /// `scale` in the SVG.
    pub scale: f64,

    /// Indicates a source color channel along the X-axis.
    ///
    /// `xChannelSelector` in the SVG.
    pub x_channel_selector: ColorChannel,

    /// Indicates a source color channel along the Y-axis.
    ///
    /// `yChannelSelector` in the SVG.
    pub y_channel_selector: ColorChannel,
}


/// A flood filter primitive.
///
/// `feFlood` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct FeFlood {
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
pub struct FeGaussianBlur {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A standard deviation along the X-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_x: PositiveNumber,

    /// A standard deviation along the Y-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_y: PositiveNumber,
}


/// An image filter primitive.
///
/// `feImage` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeImage {
    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,

    /// Rendering method.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub data: FeImageKind,
}


/// A merge filter primitive.
///
/// `feMerge` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeMerge {
    /// List of input layers that should be merged.
    ///
    /// List of `feMergeNode`'s in the SVG.
    pub inputs: Vec<FilterInput>,
}


/// A morphology filter primitive.
///
/// `feMorphology` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeMorphology {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A filter operator.
    ///
    /// `operator` in the SVG.
    pub operator: FeMorphologyOperator,

    /// A filter radius along the X-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_x: PositiveNumber,

    /// A filter radius along the Y-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_y: PositiveNumber,
}


/// An offset filter primitive.
///
/// `feOffset` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeOffset {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// The amount to offset the input graphic along the X-axis.
    pub dx: f64,

    /// The amount to offset the input graphic along the Y-axis.
    pub dy: f64,
}


/// A specular lighting filter primitive.
///
/// `feSpecularLighting` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeSpecularLighting {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,

    /// A surface scale.
    ///
    /// `surfaceScale` in the SVG.
    pub surface_scale: f64,

    /// A specular constant.
    ///
    /// `specularConstant` in the SVG.
    pub specular_constant: f64,

    /// A specular exponent.
    ///
    /// Should be in 1..128 range.
    ///
    /// `specularExponent` in the SVG.
    pub specular_exponent: f64,

    /// A lighting color.
    ///
    /// `lighting-color` in the SVG.
    pub lighting_color: Color,

    /// A light source.
    pub light_source: FeLightSource,
}


/// A tile filter primitive.
///
/// `feTile` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeTile {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,
}


/// A turbulence generation filter primitive.
///
/// `feTurbulence` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct FeTurbulence {
    /// Identifies the base frequency for the noise function.
    ///
    /// `baseFrequency` in the SVG.
    pub base_frequency: Point<PositiveNumber>,

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
    pub kind: FeTurbulenceKind,
}


/// A light source kind.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum FeLightSource {
    FeDistantLight(FeDistantLight),
    FePointLight(FePointLight),
    FeSpotLight(FeSpotLight),
}


/// A distant light source.
///
/// `feDistantLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct FeDistantLight {
    /// Direction angle for the light source on the XY plane (clockwise),
    /// in degrees from the x axis.
    ///
    /// `azimuth` in the SVG.
    pub azimuth: f64,

    /// Direction angle for the light source from the XY plane towards the z axis, in degrees.
    ///
    /// `elevation` in the SVG.
    pub elevation: f64,
}


/// A point light source.
///
/// `fePointLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct FePointLight {
    /// X location for the light source.
    ///
    /// `x` in the SVG.
    pub x: f64,

    /// Y location for the light source.
    ///
    /// `y` in the SVG.
    pub y: f64,

    /// Z location for the light source.
    ///
    /// `z` in the SVG.
    pub z: f64,
}


/// A spot light source.
///
/// `feSpotLight` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct FeSpotLight {
    /// X location for the light source.
    ///
    /// `x` in the SVG.
    pub x: f64,

    /// Y location for the light source.
    ///
    /// `y` in the SVG.
    pub y: f64,

    /// Z location for the light source.
    ///
    /// `z` in the SVG.
    pub z: f64,

    /// X point at which the light source is pointing.
    ///
    /// `pointsAtX` in the SVG.
    pub points_at_x: f64,

    /// Y point at which the light source is pointing.
    ///
    /// `pointsAtY` in the SVG.
    pub points_at_y: f64,

    /// Z point at which the light source is pointing.
    ///
    /// `pointsAtZ` in the SVG.
    pub points_at_z: f64,

    /// Exponent value controlling the focus for the light source.
    ///
    /// `specularExponent` in the SVG.
    pub specular_exponent: PositiveNumber,

    /// A limiting cone which restricts the region where the light is projected.
    ///
    /// `limitingConeAngle` in the SVG.
    pub limiting_cone_angle: Option<f64>,
}
