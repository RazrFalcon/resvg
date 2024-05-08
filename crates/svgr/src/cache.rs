use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash, Hasher},
};

use lru::LruCache;
use tiny_skia::Pixmap;

#[derive(Debug)]
struct SvgrCacheInternal<HashBuilder: BuildHasher = ahash::RandomState> {
    lru: LruCache<u64, Pixmap>,
    hash_builder: HashBuilder,
}

/// Defines rendering LRU cache. Each individual node and group will be cached separately.
/// Make sure that in most cases it will require saving of the whole canvas which may lead to significant memory usage.
/// So it is recommended to set the cache size to a reasonable value.
///
/// Pass &mut SvgrCache::none() if you don't need caching.
#[derive(Debug)]
pub struct SvgrCache<RandomState: BuildHasher = ahash::RandomState>(
    Option<SvgrCacheInternal<RandomState>>,
);

impl SvgrCache {
    /// Creates a new cache with the specified capacity.
    /// If capacity <= 0 then cache is disabled and this struct does not allocate.
    /// Uses `ahash` as a hasher, if you want to specify custom hasher user `new_with_hasher` fn.
    pub fn new(size: usize) -> Self {
        Self::new_with_hasher(size, ahash::RandomState::default())
    }
}

impl<THashBuilder: BuildHasher + Default> SvgrCache<THashBuilder> {
    /// Creates a no cache value. Basically an Option::None.
    pub fn none() -> Self {
        Self(None)
    }

    /// Creates a new cache with the specified capacity.
    /// If capacity <= 0 then cache is disabled.
    pub fn new_with_hasher(size: usize, hasher: THashBuilder) -> Self {
        if size > 0 {
            Self(Some(SvgrCacheInternal {
                lru: LruCache::new(std::num::NonZeroUsize::new(size).unwrap()),
                hash_builder: hasher,
            }))
        } else {
            Self::none()
        }
    }

    fn lru(&mut self) -> Option<&mut LruCache<u64, Pixmap>> {
        self.0.as_mut().map(|cache| &mut cache.lru)
    }

    fn hash(&self, node: &impl Hash) -> Option<u64> {
        let cache = self.0.as_ref()?;

        let mut hasher = cache.hash_builder.build_hasher();
        node.hash(&mut hasher);
        Some(Hasher::finish(&hasher))
    }

    /// Creates sub pixmap that will be cached itself within a canvas cache. Guarantees empty canvas within closure.  
    pub(crate) fn with_subpixmap_cache<'a>(
        &'a mut self,
        node: &impl Hash,
        mut f: impl FnMut(&'a mut Self) -> Option<(Pixmap, &'a mut Self)>,
    ) -> Option<Cow<'a, Pixmap>> {
        if let None = self.0 {
            println!("Cache is disabled");
            return f(self).map(|(value, _)| Cow::Owned(value));
        }

        let hash = self.hash(node)?;
        let mut cache_ref = self;

        if !cache_ref.lru()?.contains(&hash) {
            let (value, cache_back) = { f(cache_ref)? };

            // we basically passing down the mutable ref and getting it back
            // this is a primitive way to achieve recurisve mutable borrowing
            // without any overhead of Rc or RefCell
            cache_ref = cache_back;
            cache_ref.lru()?.put(hash, value);
        }

        let pixmap = cache_ref.lru()?.peek(&hash)?;
        return Some(Cow::Borrowed(pixmap));
    }
}

/// Removes transparent borders from the image leaving only a tight bbox content.
///
/// Detects graphics element bbox on the raster images in absolute coordinates.
///
/// The current implementation is extremely simple and fairly slow.
/// Ideally, we should calculate the absolute bbox based on the current transform and bbox.
/// But because of anti-aliasing, float precision and especially stroking,
/// this can be fairly complicated and error-prone.
/// So for now we're using this method.
pub fn trim_transparency(
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<(i32, i32, tiny_skia::Pixmap)> {
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let mut min_x = pixmap.width() as i32;
    let mut min_y = pixmap.height() as i32;
    let pixels = pixmap.data_mut();
    let mut max_x = 0;
    let mut max_y = 0;

    let first_non_zero = {
        let max_safe_index = pixels.len() / 8;

        // Find first non-zero byte by looking at 8 bytes a time. If not found
        // checking the remaining bytes. This is a lot faster than checking one
        // byte a time.
        (0..max_safe_index)
            .position(|i| {
                let idx = i * 8;
                u64::from_ne_bytes((&pixels[idx..(idx + 8)]).try_into().unwrap()) != 0
            })
            .map_or_else(
                || ((max_safe_index * 8)..pixels.len()).position(|i| pixels[i] != 0),
                |i| Some(i * 8),
            )
    };

    // We skip all the transparent pixels at the beginning of the image. It's
    // very likely that transparent pixels all have rgba(0, 0, 0, 0) so skipping
    // zero bytes can be used as a quick optimization.
    // If the entire image is transparent, we don't need to continue.
    if first_non_zero.is_some() {
        let get_alpha = |x, y| pixels[((width * y + x) * 4 + 3) as usize];

        // Find the top boundary.
        let start_y = first_non_zero.unwrap() as i32 / 4 / width;
        'top: for y in start_y..height {
            for x in 0..width {
                if get_alpha(x, y) != 0 {
                    min_x = x;
                    max_x = x;
                    min_y = y;
                    max_y = y;
                    break 'top;
                }
            }
        }

        // Find the bottom boundary.
        'bottom: for y in (max_y..height).rev() {
            for x in 0..width {
                if get_alpha(x, y) != 0 {
                    max_y = y;
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    break 'bottom;
                }
            }
        }

        // Find the left boundary.
        'left: for x in 0..min_x {
            for y in min_y..max_y {
                if get_alpha(x, y) != 0 {
                    min_x = x;
                    break 'left;
                }
            }
        }

        // Find the right boundary.
        'right: for x in (max_x..width).rev() {
            for y in min_y..max_y {
                if get_alpha(x, y) != 0 {
                    max_x = x;
                    break 'right;
                }
            }
        }
    }

    // Expand in all directions by 1px.
    min_x = (min_x - 1).max(0);
    min_y = (min_y - 1).max(0);
    max_x = (max_x + 2).min(pixmap.width() as i32);
    max_y = (max_y + 2).min(pixmap.height() as i32);

    if min_x < max_x && min_y < max_y {
        let rect = tiny_skia::IntRect::from_ltrb(min_x, min_y, max_x, max_y)?;
        let pixmap = pixmap.as_ref().clone_rect(rect)?;
        Some((min_x, min_y, pixmap))
    } else {
        Some((0, 0, pixmap.to_owned()))
    }
}
