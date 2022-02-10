//! ImageHrefResolver specifies the way to parse `<image>` elements `xlink:href` value.

pub mod default_resolver;

use crate::ImageKind;
use std::{
    fmt::{Debug, Formatter, Result},
    sync::Arc,
};

/// Functions that accept various representations of `xlink:href` value of the `<image`>
/// element and return ImageKind that holds reference to the image buffer determined by these functions.
#[derive(Clone, Debug)]
pub struct ImageHrefResolver<'a> {
    /// Resolver function that will be used if `xlink:href` is a DataUrl with encoded base64 string.
    pub resolve_data: Box<dyn ImageHrefDataResolver + 'a>,
    /// Resolver function that will be used to handle arbitrary string in `xlink:href`.
    pub resolve_string: Box<dyn ImageHrefStringResolver + 'a>,
}

/// Image `href` resolver that accepts mime type string and reference to decoded base64 image data.
pub trait ImageHrefDataResolver: Fn(&str, Arc<Vec<u8>>) -> Option<ImageKind> {
    /// Allow ImageHrefResolver to derive Clone
    fn clone_box<'a>(&self) -> Box<dyn 'a + ImageHrefDataResolver>
    where
        Self: 'a;
}

impl<F: Fn(&str, Arc<Vec<u8>>) -> Option<ImageKind> + Clone> ImageHrefDataResolver for F {
    fn clone_box<'a>(&self) -> Box<dyn 'a + ImageHrefDataResolver>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + ImageHrefDataResolver> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl<'a> Debug for Box<dyn 'a + ImageHrefDataResolver> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str("ImageHrefDataResolver function (..)")
    }
}

/// Image `href` resolver that accepts whole `href` value as string.
pub trait ImageHrefStringResolver: Fn(&str) -> Option<ImageKind> {
    /// Allow ImageHrefResolver to derive Clone
    fn clone_box<'a>(&self) -> Box<dyn 'a + ImageHrefStringResolver>
    where
        Self: 'a;
}

impl<F: Fn(&str) -> Option<ImageKind> + Clone> ImageHrefStringResolver for F {
    fn clone_box<'a>(&self) -> Box<dyn 'a + ImageHrefStringResolver>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a> Clone for Box<dyn 'a + ImageHrefStringResolver> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl<'a> Debug for Box<dyn 'a + ImageHrefStringResolver> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str("ImageHrefStringResolver function (..)")
    }
}
