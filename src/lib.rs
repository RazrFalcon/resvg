// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[resvg](https://github.com/RazrFalcon/resvg) is an SVG rendering library.
*/

#![doc(html_root_url = "https://docs.rs/resvg/0.12.0")]

#![warn(missing_docs)]

pub use rgb::RGBA8;
pub use usvg::ScreenSize;

use usvg::NodeExt;
use log::warn;

#[macro_use] mod macros;
mod clip;
mod filter;
mod image;
mod layers;
mod mask;
mod paint_server;
mod path;
mod render;


/// A raster image that contains rendering results.
///
/// Unpremultiplied RGBA color type is used.
#[derive(Clone)]
pub struct Image {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl Image {
    fn from_pixmap(pixmap: tiny_skia::Pixmap) -> Self {
        use rgb::FromSlice;

        let width = pixmap.width();
        let height = pixmap.height();

        let mut data = pixmap.take();
        svgfilters::demultiply_alpha(data.as_rgba_mut());

        Image {
            data,
            width,
            height,
        }
    }

    /// Returns the image width.
    ///
    /// Newer zero.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the image height.
    ///
    /// Newer zero.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the image size.
    pub fn size(&self) -> ScreenSize {
        ScreenSize::new(self.width(), self.height()).unwrap()
    }

    /// Returns the image content as `u8` slice.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the underlying data.
    pub fn take(self) -> Vec<u8> {
        self.data
    }

    /// Save the image as PNG at a provided path.
    pub fn save_png<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), png::EncodingError> {
        let file = std::fs::File::create(path)?;
        let ref mut w = std::io::BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, self.width(), self.height());
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data())
    }
}


/// Renders an SVG to image.
pub fn render(
    tree: &usvg::Tree,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<Image> {
    let (mut pixmap, size)
        = render::create_root_image(tree.svg_node().size.to_screen_size(), fit_to, background)?;
    let mut canvas = tiny_skia::Canvas::from(pixmap.as_mut());
    render::render_to_canvas(tree, size, &mut canvas);
    Some(Image::from_pixmap(pixmap))
}

/// Renders an SVG node to image.
pub fn render_node(
    node: &usvg::Node,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<Image> {
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
        bbox
    } else {
        warn!("Node '{}' has zero size.", node.id());
        return None;
    };

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let (mut pixmap, img_size)
        = render::create_root_image(node_bbox.size().to_screen_size(), fit_to, background)?;

    let mut canvas = tiny_skia::Canvas::from(pixmap.as_mut());
    render::render_node_to_canvas(node, vbox, img_size, &mut render::RenderState::Ok, &mut canvas);
    Some(Image::from_pixmap(pixmap))
}
