use std::{
    hash::{Hash, Hasher},
    mem,
};

use tiny_skia_path::{NonZeroRect, Path, PathVerb, Point, Rect, Transform};

pub trait CustomHash {
    fn custom_hash<H: Hasher>(&self, state: &mut H);

    fn custom_hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        let newlen = mem::size_of_val(data);
        let ptr = data.as_ptr() as *const u8;
        // SAFETY: `ptr` is valid and aligned, as this macro is only used
        // for numeric primitives which have no padding. The new slice only
        // spans across `data` and is never mutated, and its total size is the
        // same as the original `data` so it can't be over `isize::MAX`.
        state.write(unsafe { core::slice::from_raw_parts(ptr, newlen) })
    }
}

impl CustomHash for NonZeroRect {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        self.x().to_bits().hash(state);
        self.y().to_bits().hash(state);
        self.width().to_bits().hash(state);
        self.height().to_bits().hash(state);
    }
}

impl CustomHash for Transform {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        self.sx.to_bits().hash(state);
        self.kx.to_bits().hash(state);
        self.ky.to_bits().hash(state);
        self.sy.to_bits().hash(state);
        self.tx.to_bits().hash(state);
        self.ty.to_bits().hash(state);
    }
}

impl CustomHash for Rect {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        self.x().to_bits().hash(state);
        self.y().to_bits().hash(state);
        self.width().to_bits().hash(state);
        self.height().to_bits().hash(state);
    }
}

impl CustomHash for PathVerb {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PathVerb::Move => 0.hash(state),
            PathVerb::Line => 1.hash(state),
            PathVerb::Quad => 2.hash(state),
            PathVerb::Cubic => 3.hash(state),
            PathVerb::Close => 4.hash(state),
        }
    }
}

impl CustomHash for Point {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

impl CustomHash for Path {
    fn custom_hash<H: Hasher>(&self, state: &mut H) {
        CustomHash::custom_hash_slice(self.verbs(), state);
        CustomHash::custom_hash_slice(self.points(), state);
        self.bounds().custom_hash(state);
    }
}
