// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ops::Deref;

use crate::geom::*;
use super::attributes::*;

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
    pub segments: Vec<PathSegment>,
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
            segments: Vec::new(),
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
    FeComponentTransfer(FeComponentTransfer),
    FeComposite(FeComposite),
    FeFlood(FeFlood),
    FeGaussianBlur(FeGaussianBlur),
    FeImage(FeImage),
    FeMerge(FeMerge),
    FeOffset(FeOffset),
    FeTile(FeTile),
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

impl TransferFunction {
    /// Applies a transfer function to a provided color component.
    ///
    /// Requires a non-premultiplied color component.
    pub fn apply(&self, c: u8) -> u8 {
        (f64_bound(0.0, self.apply_impl(c as f64 / 255.0), 1.0) * 255.0) as u8
    }

    fn apply_impl(&self, c: f64) -> f64 {
        use std::cmp;

        match self {
            TransferFunction::Identity => {
                c
            }
            TransferFunction::Table(ref values) => {
                if values.is_empty() {
                    return c;
                }

                let n = values.len() - 1;
                let k = (c * (n as f64)).floor() as usize;
                let k = cmp::min(k, n);
                if k == n {
                    return values[k];
                }

                let vk = values[k];
                let vk1 = values[k + 1];
                let k = k as f64;
                let n = n as f64;

                vk + (c - k / n) * n * (vk1 - vk)
            }
            TransferFunction::Discrete(ref values) => {
                if values.is_empty() {
                    return c;
                }

                let n = values.len();
                let k = (c * (n as f64)).floor() as usize;

                values[cmp::min(k, n - 1)]
            }
            TransferFunction::Linear { slope, intercept } => {
                slope * c + intercept
            }
            TransferFunction::Gamma { amplitude, exponent, offset } => {
                amplitude * c.powf(*exponent) + offset
            }
        }
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_size() {
        assert!(std::mem::size_of::<NodeKind>() <= 256);
    }
}
