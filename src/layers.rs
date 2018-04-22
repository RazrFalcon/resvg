// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cell::RefCell;
use std::rc::Rc;

use {
    ScreenSize,
    Options,
};


/// Stack of image layers.
///
/// Instead of creating a new layer each time we need one,
/// we are reusing an existing one.
pub struct Layers<'a, T> {
    d: Vec<Rc<RefCell<T>>>,
    idx: usize,
    img_size: ScreenSize,
    opt: &'a Options,
    new_img_fn: Box<Fn(ScreenSize, &Options) -> Option<T>>,
    clear_img_fn: Box<Fn(&mut T)>,
}

impl<'a, T> Layers<'a, T> {
    /// Creates `Layers`.
    pub fn new<F1, F2>(
        img_size: ScreenSize,
        opt: &'a Options,
        new_img_fn: F1,
        clear_img_fn: F2,
    ) -> Self
        where F1: Fn(ScreenSize, &Options) -> Option<T> + 'static,
              F2: Fn(&mut T) + 'static,
    {
        Layers {
            d: Vec::new(),
            idx: 0,
            img_size,
            opt,
            new_img_fn: Box::new(new_img_fn),
            clear_img_fn: Box::new(clear_img_fn),
        }
    }

    /// Returns a first free layer to draw on.
    ///
    /// - If there are no free layers - will create a new one.
    /// - If there is a free layer - it will clear it before return.
    pub fn get(&mut self) -> Option<Rc<RefCell<T>>> {
        if self.idx == self.d.len() {
            match (self.new_img_fn)(self.img_size, self.opt) {
                Some(img) => {
                    self.d.push(Rc::new(RefCell::new(img)));
                    self.idx += 1;
                    Some(self.d[self.idx - 1].clone())
                }
                None => {
                    None
                }
            }
        } else {
            {
                let img = self.d[self.idx].clone();
                (self.clear_img_fn)(&mut img.borrow_mut());
            }

            self.idx += 1;
            Some(self.d[self.idx - 1].clone())
        }
    }

    /// Marks the last layer as free.
    pub fn release(&mut self) {
        self.idx -= 1;
    }
}

impl<'a, T> Drop for Layers<'a, T> {
    fn drop(&mut self) {
        debug_assert!(self.idx == 0);
    }
}
