// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        USvg(usvg::Error, usvg::ErrorKind) #[doc = "'usvg' errors"];
    }

    foreign_links {
        Io(::std::io::Error) #[doc = "io errors"];
        UTF8(::std::string::FromUtf8Error) #[doc = "UTF-8 errors"];
    }

    errors {
        /// Failed to allocate an image.
        ///
        /// Probably because it's too big or there is not enough memory.
        NoCanvas {
            display("the main canvas creation failed")
        }
    }
}
