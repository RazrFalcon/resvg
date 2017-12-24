// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        SvgDom(svgdom::Error, svgdom::ErrorKind) #[doc = "'svgdom' errors"];
    }

    foreign_links {
        Io(::std::io::Error) #[doc = "io errors"];
        UTF8(::std::string::FromUtf8Error) #[doc = "UTF-8 errors"];
    }

    errors {
        /// Failed to find an SVG size.
        ///
        /// SVG size must be explicitly defined.
        /// Automatic image size determination is not supported.
        SizeDeterminationUnsupported {
            display("file doesn't have 'width', 'height' and 'viewBox' attributes. \
                     Automatic image size determination is not supported")
        }

        /// The `svg` node is missing.
        ///
        /// This error indicates an error in the preprocessor.
        MissingSvgNode {
            display("the root svg node is missing")
        }

        /// SVG size is not resolved.
        ///
        /// This error indicates an error in the preprocessor.
        InvalidSize {
            display("invalid SVG size")
        }

        /// An invalid `viewBox` attribute content.
        InvalidViewBox(s: String) {
            display("invalid 'viewBox' attribute value: '{}'", s)
        }

        /// Failed to allocate an image.
        ///
        /// Probably because it's too big or there is not enough memory.
        NoCanvas {
            display("the main canvas creation failed")
        }

        /// An invalid file extension.
        ///
        /// The extension should be 'svg' or 'svgz' in any case.
        InvalidFileExtension {
            display("invalid file extension")
        }
    }
}
