use crate::{Error, Stream};

/// Representation of the `align` value of the [`preserveAspectRatio`] attribute.
///
/// [`preserveAspectRatio`]: https://www.w3.org/TR/SVG11/coords.html#PreserveAspectRatioAttribute
#[allow(missing_docs)]
#[derive(Clone, Hash, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    None,
    XMinYMin,
    XMidYMin,
    XMaxYMin,
    XMinYMid,
    XMidYMid,
    XMaxYMid,
    XMinYMax,
    XMidYMax,
    XMaxYMax,
}

impl quote::ToTokens for Align {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Align::None => quote::quote! {Align::None},
            Align::XMinYMin => quote::quote! {svgrtypes::Align::XMinYMin},
            Align::XMidYMin => quote::quote! {svgrtypes::Align::XMidYMin},
            Align::XMaxYMin => quote::quote! {svgrtypes::Align::XMaxYMin},
            Align::XMinYMid => quote::quote! {svgrtypes::Align::XMinYMid},
            Align::XMidYMid => quote::quote! {svgrtypes::Align::XMidYMid},
            Align::XMaxYMid => quote::quote! {svgrtypes::Align::XMaxYMid},
            Align::XMinYMax => quote::quote! {svgrtypes::Align::XMinYMax},
            Align::XMidYMax => quote::quote! {svgrtypes::Align::XMidYMax},
            Align::XMaxYMax => quote::quote! {svgrtypes::Align::XMaxYMax},
        }
        .to_tokens(tokens)
    }
}

/// Representation of the [`preserveAspectRatio`] attribute.
///
/// SVG 2 removed the `defer` keyword, but we still support it.
///
/// [`preserveAspectRatio`]: https://www.w3.org/TR/SVG11/coords.html#PreserveAspectRatioAttribute
#[derive(Clone, Hash, Copy, PartialEq, Eq, Debug)]
pub struct AspectRatio {
    /// `<defer>` value.
    ///
    /// Set to `true` when `defer` value is present.
    pub defer: bool,
    /// `<align>` value.
    pub align: Align,
    /// `<meetOrSlice>` value.
    ///
    /// - Set to `true` when `slice` value is present.
    /// - Set to `false` when `meet` value is present or value is not set at all.
    pub slice: bool,
}

impl quote::ToTokens for AspectRatio {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            defer,
            slice,
            align,
        } = self;

        quote::quote! {
            svgrtypes::AspectRatio {
                defer: #defer,
                slice: #slice,
                align: #align,
            }
        }
        .to_tokens(tokens)
    }
}

impl std::str::FromStr for AspectRatio {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let mut s = Stream::from(text);

        s.skip_spaces();

        let defer = s.starts_with(b"defer");
        if defer {
            s.advance(5);
            s.consume_byte(b' ')?;
            s.skip_spaces();
        }

        let start = s.pos();
        let align = s.consume_ascii_ident();
        let align = match align {
            "none" => Align::None,
            "xMinYMin" => Align::XMinYMin,
            "xMidYMin" => Align::XMidYMin,
            "xMaxYMin" => Align::XMaxYMin,
            "xMinYMid" => Align::XMinYMid,
            "xMidYMid" => Align::XMidYMid,
            "xMaxYMid" => Align::XMaxYMid,
            "xMinYMax" => Align::XMinYMax,
            "xMidYMax" => Align::XMidYMax,
            "xMaxYMax" => Align::XMaxYMax,
            _ => return Err(Error::UnexpectedData(s.calc_char_pos_at(start))),
        };

        s.skip_spaces();

        let mut slice = false;
        if !s.at_end() {
            let start = s.pos();
            let v = s.consume_ascii_ident();
            match v {
                "meet" => {}
                "slice" => slice = true,
                "" => {}
                _ => return Err(Error::UnexpectedData(s.calc_char_pos_at(start))),
            };
        }

        Ok(AspectRatio {
            defer,
            align,
            slice,
        })
    }
}

impl Default for AspectRatio {
    #[inline]
    fn default() -> Self {
        AspectRatio {
            defer: false,
            align: Align::XMidYMid,
            slice: false,
        }
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    macro_rules! test {
        ($name:ident, $text:expr, $result:expr) => (
            #[test]
            fn $name() {
                let v = AspectRatio::from_str($text).unwrap();
                assert_eq!(v, $result);
            }
        )
    }

    test!(parse_1, "none", AspectRatio {
        defer: false,
        align: Align::None,
        slice: false,
    });

    test!(parse_2, "defer none", AspectRatio {
        defer: true,
        align: Align::None,
        slice: false,
    });

    test!(parse_3, "xMinYMid", AspectRatio {
        defer: false,
        align: Align::XMinYMid,
        slice: false,
    });

    test!(parse_4, "xMinYMid slice", AspectRatio {
        defer: false,
        align: Align::XMinYMid,
        slice: true,
    });

    test!(parse_5, "xMinYMid meet", AspectRatio {
        defer: false,
        align: Align::XMinYMid,
        slice: false,
    });
}
