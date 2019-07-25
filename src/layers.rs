// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Rect, ScreenSize};
use crate::backend_utils::BlendMode;


pub trait Image: Sized {
    fn new(size: ScreenSize) -> Option<Self>;
    fn clear(&mut self);
}

pub struct Layer<T> {
    pub ts: usvg::Transform,
    pub blend_mode: BlendMode,
    pub clip_rect: Option<Rect>,
    pub img: T,
}

/// A stack-like container which doesn't deallocate a value on `pop()`.
///
/// A layer allocation is very expensive, so instead of deallocating
/// a layer on a `pop()` call, we simply decrementing the internal stack length value.
pub struct Layers<T> {
    img_size: ScreenSize,
    layers: Vec<Option<Layer<T>>>,

    /// Amount of used layers.
    ///
    /// Can be smaller than amount of allocated layers.
    used_layers: usize,
}

impl<T: Image> Layers<T> {
    /// Creates `Layers`.
    pub fn new(img_size: ScreenSize) -> Self {
        Layers {
            img_size,
            layers: Vec::new(),
            used_layers: 0,
        }
    }

    #[cfg(any(feature = "qt-backend", feature = "skia-backend", feature = "skia-backend-bindings"))]
    pub fn is_empty(&self) -> bool {
        self.used_layers == 0
    }

    pub fn img_size(&self) -> ScreenSize {
        self.img_size
    }

    /// Pushes a new layer.
    pub fn push(&mut self) -> Option<()> {
        if self.layers.len() == self.used_layers {
            // If all layers are used - allocate a new one.

            let img = T::new(self.img_size)?;
            self.used_layers += 1;
            self.layers.push(Some(Layer {
                ts: usvg::Transform::default(),
                blend_mode: BlendMode::default(),
                clip_rect: None,
                img,
            }))
        } else {
            // If we have a free layer - clear it and mark as current one.

            let layer = self.layers[self.used_layers].as_mut().unwrap();
            layer.ts = usvg::Transform::default();
            layer.blend_mode = BlendMode::default();
            layer.clip_rect = None;
            layer.img.clear();

            self.used_layers += 1;
        }

        Some(())
    }

    /// Pushes an existing layer.
    ///
    /// Unlike `push()`, doesn't increment the layers count.
    ///
    /// Must be executed after `pop()`.
    pub fn push_back(&mut self, layer: Layer<T>) {
        self.layers[self.used_layers] = Some(layer);
    }

    /// Pops the last layer.
    pub fn pop(&mut self) -> Option<Layer<T>> {
        if self.used_layers > 0 {
            self.used_layers -= 1;
            let last = std::mem::replace(&mut self.layers[self.used_layers], None);
            Some(last.unwrap())
        } else {
            None
        }
    }

    /// Returns the current layer.
    pub fn current(&self) -> Option<&Layer<T>> {
        if self.used_layers > 0 {
            self.layers.get(self.used_layers - 1).map(|v| v.as_ref().unwrap())
        } else {
            None
        }
    }

    /// Returns the current layer.
    pub fn current_mut(&mut self) -> Option<&mut Layer<T>> {
        if self.used_layers > 0 {
            self.layers.get_mut(self.used_layers - 1).map(|v| v.as_mut().unwrap())
        } else {
            None
        }
    }
}
