// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg;

/// Errors list.
#[derive(Fail, Debug)]
pub enum Error {
    /// Failed to allocate an image.
    ///
    /// Probably because it's too big or there is not enough memory.
    #[fail(display = "the main canvas creation failed")]
    NoCanvas,

    /// `usvg` file read errors.
    #[fail(display = "{}", _0)]
    FileReadError(usvg::FileReadError),
}

impl From<usvg::FileReadError> for Error {
    fn from(value: usvg::FileReadError) -> Error {
        Error::FileReadError(value)
    }
}

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub(crate) type Result<T> = ::std::result::Result<T, Error>;
