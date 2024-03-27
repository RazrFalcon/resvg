use crate::{Error, Length, LengthUnit, Stream};

/// List of all SVG directional positions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectionalPosition {
    /// The `top` position.
    Top,
    /// The `center` position.
    Center,
    /// The `bottom` position.
    Bottom,
    /// The `right` position.
    Right,
    /// The `left` position.
    Left,
}

impl DirectionalPosition {
    /// Checks whether the value can be a horizontal position.
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        match self {
            DirectionalPosition::Center
            | DirectionalPosition::Left
            | DirectionalPosition::Right => true,
            _ => false,
        }
    }

    /// Checks whether the value can be a vertical position.
    #[inline]
    pub fn is_vertical(&self) -> bool {
        match self {
            DirectionalPosition::Center
            | DirectionalPosition::Top
            | DirectionalPosition::Bottom => true,
            _ => false,
        }
    }
}

impl From<DirectionalPosition> for Length {
    fn from(value: DirectionalPosition) -> Self {
        match value {
            DirectionalPosition::Left | DirectionalPosition::Top => {
                Length::new(0.0, LengthUnit::Percent)
            }
            DirectionalPosition::Right | DirectionalPosition::Bottom => {
                Length::new(100.0, LengthUnit::Percent)
            }
            DirectionalPosition::Center => Length::new(50.0, LengthUnit::Percent),
        }
    }
}

impl std::str::FromStr for DirectionalPosition {
    type Err = Error;

    #[inline]
    fn from_str(text: &str) -> Result<Self, Error> {
        let mut s = Stream::from(text);
        let dir_pos = s.parse_directional_position()?;

        if !s.at_end() {
            return Err(Error::UnexpectedData(s.calc_char_pos()));
        }

        Ok(dir_pos)
    }
}

impl<'a> Stream<'a> {
    /// Parses a directional position [`left`, `center`, `right`, `bottom`, `top`] from the stream.
    pub fn parse_directional_position(&mut self) -> Result<DirectionalPosition, Error> {
        self.skip_spaces();

        if self.starts_with(b"left") {
            self.advance(4);
            return Ok(DirectionalPosition::Left);
        } else if self.starts_with(b"right") {
            self.advance(5);
            return Ok(DirectionalPosition::Right);
        } else if self.starts_with(b"top") {
            self.advance(3);
            return Ok(DirectionalPosition::Top);
        } else if self.starts_with(b"bottom") {
            self.advance(6);
            return Ok(DirectionalPosition::Bottom);
        } else if self.starts_with(b"center") {
            self.advance(6);
            return Ok(DirectionalPosition::Center);
        } else {
            return Err(Error::InvalidString(
                vec![
                    self.slice_tail().to_string(),
                    "left".to_string(),
                    "right".to_string(),
                    "top".to_string(),
                    "bottom".to_string(),
                    "center".to_string(),
                ],
                self.calc_char_pos(),
            ));
        }
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    macro_rules! test_p {
        ($name:ident, $text:expr, $result:expr) => (
            #[test]
            fn $name() {
                assert_eq!(DirectionalPosition::from_str($text).unwrap(), $result);
            }
        )
    }

    test_p!(parse_1,  "left", DirectionalPosition::Left);
    test_p!(parse_2,  "right", DirectionalPosition::Right);
    test_p!(parse_3,  "center", DirectionalPosition::Center);
    test_p!(parse_4,  "top", DirectionalPosition::Top);
    test_p!(parse_5,  "bottom", DirectionalPosition::Bottom);

    #[test]
    fn parse_6() {
        let mut s = Stream::from("left,");
        assert_eq!(s.parse_directional_position().unwrap(), DirectionalPosition::Left);
    }

    #[test]
    fn parse_7() {
        let mut s = Stream::from("left ,");
        assert_eq!(s.parse_directional_position().unwrap(), DirectionalPosition::Left);
    }

    #[test]
    fn parse_16() {
        let mut s = Stream::from("left center");
        assert_eq!(s.parse_directional_position().unwrap(), DirectionalPosition::Left);
    }

    #[test]
    fn err_1() {
        let mut s = Stream::from("something");
        assert_eq!(s.parse_directional_position().unwrap_err().to_string(),
                   "expected 'left', 'right', 'top', 'bottom', 'center' not 'something' at position 1");
    }
}
