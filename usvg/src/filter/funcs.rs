// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgtypes::Length;

use crate::svgtree::{self, AId};
use crate::{converter, Opacity, PositiveNumber, Units, Color};
use super::{ColorMatrix, ColorMatrixKind, ComponentTransfer, DropShadow};
use super::{GaussianBlur, Input, Kind, TransferFunction};

#[inline(never)]
pub fn convert_grayscale(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Matrix(vec![
            (0.2126 + 0.7874 * (1.0 - amount)),
            (0.7152 - 0.7152 * (1.0 - amount)),
            (0.0722 - 0.0722 * (1.0 - amount)),
            0.0,
            0.0,

            (0.2126 - 0.2126 * (1.0 - amount)),
            (0.7152 + 0.2848 * (1.0 - amount)),
            (0.0722 - 0.0722 * (1.0 - amount)),
            0.0,
            0.0,

            (0.2126 - 0.2126 * (1.0 - amount)),
            (0.7152 - 0.7152 * (1.0 - amount)),
            (0.0722 + 0.9278 * (1.0 - amount)),
            0.0,
            0.0,

            0.0, 0.0, 0.0, 1.0, 0.0,
        ]),
    })
}

#[inline(never)]
pub fn convert_sepia(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Matrix(vec![
            (0.393 + 0.607 * (1.0 - amount)),
            (0.769 - 0.769 * (1.0 - amount)),
            (0.189 - 0.189 * (1.0 - amount)),
            0.0,
            0.0,

            (0.349 - 0.349 * (1.0 - amount)),
            (0.686 + 0.314 * (1.0 - amount)),
            (0.168 - 0.168 * (1.0 - amount)),
            0.0,
            0.0,

            (0.272 - 0.272 * (1.0 - amount)),
            (0.534 - 0.534 * (1.0 - amount)),
            (0.131 + 0.869 * (1.0 - amount)),
            0.0,
            0.0,

            0.0, 0.0, 0.0, 1.0, 0.0,
        ]),
    })
}

#[inline(never)]
pub fn convert_saturate(amount: f64) -> Kind {
    let amount = PositiveNumber::new(amount.max(0.0));
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Saturate(amount),
    })
}

#[inline(never)]
pub fn convert_hue_rotate(amount: svgtypes::Angle) -> Kind {
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::HueRotate(amount.to_degrees()),
    })
}

#[inline(never)]
pub fn convert_invert(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_g: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_b: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
pub fn convert_opacity(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Identity,
        func_g: TransferFunction::Identity,
        func_b: TransferFunction::Identity,
        func_a: TransferFunction::Table(vec![0.0, amount]),
    })
}

#[inline(never)]
pub fn convert_brightness(amount: f64) -> Kind {
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_g: TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_b: TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
pub fn convert_contrast(amount: f64) -> Kind {
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_g: TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_b: TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
pub fn convert_blur(
    node: svgtree::Node,
    std_dev: Length,
    state: &converter::State,
) -> Kind {
    let std_dev = PositiveNumber::new(
        crate::units::convert_length(std_dev, node, AId::Dx, Units::UserSpaceOnUse, state)
    );
    Kind::GaussianBlur(GaussianBlur {
        input: Input::SourceGraphic,
        std_dev_x: std_dev,
        std_dev_y: std_dev,
    })
}

#[inline(never)]
pub fn convert_drop_shadow(
    node: svgtree::Node,
    color: Option<Color>,
    dx: Length,
    dy: Length,
    std_dev: Length,
    state: &converter::State,
) -> Kind {
    let std_dev = PositiveNumber::new(
        crate::units::convert_length(std_dev, node, AId::Dx, Units::UserSpaceOnUse, state)
    );

    let color = color.unwrap_or_else(||
        node.find_attribute(AId::Color).unwrap_or_else(Color::black));

    Kind::DropShadow(DropShadow {
        input: Input::SourceGraphic,
        dx: crate::units::convert_length(dx, node, AId::Dx, Units::UserSpaceOnUse, state),
        dy: crate::units::convert_length(dy, node, AId::Dy, Units::UserSpaceOnUse, state),
        std_dev_x: std_dev,
        std_dev_y: std_dev,
        color,
        opacity: Opacity::default(),
    })
}
