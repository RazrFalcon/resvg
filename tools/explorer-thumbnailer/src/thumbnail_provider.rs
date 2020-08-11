use std::cell::RefCell;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{DWORD, UINT, ULONG};
use winapi::shared::windef::HBITMAP;
use winapi::um::objidlbase::{LPSTREAM, STATSTG};
use com::sys::{HRESULT, IID, NOERROR, S_OK, S_FALSE};
use com::co_class;
use log::{error};
use usvg::{FitTo, SystemFontDB};
use crate::interfaces::{IThumbnailProvider, IInitializeWithStream};

// {4432C229-DFD0-4B18-8C4D-F58932AF6105}
pub const CLSID_THUMBNAIL_PROVIDER_CLASS: IID = IID {
    data1: 0x4432c229,
    data2: 0xdfd0,
    data3: 0x4b18,
    data4: [0x8c, 0x4d, 0xf5, 0x89, 0x32, 0xaf, 0x61, 0x5],
};

#[co_class(implements(IThumbnailProvider, IInitializeWithStream))]
pub struct ThumbnailProvider {
    tree: RefCell<Option<usvg::Tree>>
}

impl IInitializeWithStream for ThumbnailProvider {
    unsafe fn read(&self, pstream: LPSTREAM, _grf_mode: DWORD) -> HRESULT {
        let mut stat: STATSTG = Default::default();
        let stat_res = (*pstream).Stat(&mut stat, 0);
        if stat_res != S_OK {
            error!("IStream::stat error");
            return S_FALSE;
        }
        let size = *stat.cbSize.QuadPart();
        let mut svg_data = Vec::with_capacity(size as usize);
        let mut len: ULONG = 0;
        let read_res = (*pstream).Read(svg_data.as_mut_ptr() as *mut c_void, size as u32, &mut len);
        if read_res != S_OK {
            error!("IStream::read error");
            return S_FALSE;
        }
        svg_data.set_len(len as usize);

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();

        let tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
        self.tree.replace(Some(tree));

        NOERROR
    }
}

impl IThumbnailProvider for ThumbnailProvider {
    unsafe fn get_thumbnail(&self, cx: UINT, phbmp: *mut HBITMAP, pdw_alpha: *mut UINT) -> HRESULT {
        if let Some(tree) = &*self.tree.borrow() {
            let size = tree.svg_node().size;
            let fit_to = if size.width() > size.height() {
                FitTo::Width(cx)
            } else {
                FitTo::Height(cx)
            };

            if let Some(img) = resvg::render(&tree, fit_to, None) {
                // img.save_png("C:\\the_thumbnail.png");
                *phbmp = crate::utils::to_hbitmap(img);
                *pdw_alpha = 2;
            } else {
                error!("resvg::render error");
                return S_FALSE;
            }
        } else {
            error!("SVG tree was not initialized");
            return S_FALSE;
        }
        NOERROR
    }
}

impl ThumbnailProvider {
    pub(crate) fn new() -> Box<ThumbnailProvider> {
        crate::logging::init();
        ThumbnailProvider::allocate(RefCell::new(None))
    }
}
