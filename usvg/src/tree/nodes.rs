// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ops::Deref;

// self
use geom::*;
use super::attributes::*;


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
    Marker(Marker),
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
            NodeKind::Marker(ref e) => e.id.as_str(),
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
            NodeKind::Marker(_) => Transform::default(),
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

    /// Marker.
    pub marker: Box<PathMarker>,

    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub segments: Vec<PathSegment>,
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
    pub opacity: Option<Opacity>,

    /// Element clip path.
    pub clip_path: Option<String>,

    /// Element mask.
    pub mask: Option<String>,

    /// Element filter.
    pub filter: Option<String>,
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


/// A marker element.
///
/// `marker` element in SVG.
#[derive(Clone, Debug)]
pub struct Marker {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `markerUnits` in SVG.
    pub units: MarkerUnits,

    /// Marker rectangle.
    ///
    /// `refX`, `refY`, `markerWidth` and `markerHeight` in SVG.
    pub rect: Rect,

    /// Marker viewbox.
    pub view_box: Option<ViewBox>,

    /// Marker orientation.
    ///
    /// `orient` in SVG.
    pub orientation: MarkerOrientation,

    /// Marker overflow.
    ///
    /// `overflow` in SVG.
    pub overflow: Overflow,
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
    FeComposite(FeComposite),
    FeFlood(FeFlood),
    FeGaussianBlur(FeGaussianBlur),
    FeImage(FeImage),
    FeMerge(FeMerge),
    FeOffset(FeOffset),
    FeTile(FeTile),
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


/// A blend filter primitive.
///
/// `feBlend` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeBlend {
    /// A blending mode.
    ///
    /// `mode` in the SVG.
    pub mode: FeBlendMode,

    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,
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


/// A composite filter primitive.
///
/// `feComposite` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeComposite {
    /// A compositing operation.
    ///
    /// `operator` in the SVG.
    pub operator: FeCompositeOperator,

    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,
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


/// An image filter primitive.
///
/// `feImage` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeImage {
    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,

    /// Image data.
    pub data: FeImageKind,
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
