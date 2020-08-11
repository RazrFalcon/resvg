use std::mem;
use std::ptr;
use winapi::um::wingdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS};
use winapi::shared::windef::{HBITMAP, HDC};
use winapi::um::wingdi::CreateDIBSection;

pub unsafe fn to_hbitmap(img: resvg::Image) -> HBITMAP {
    let hdc: HDC = ptr::null_mut();
    let mut bmi: BITMAPINFO = Default::default();
    bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    bmi.bmiHeader.biWidth = img.width() as i32;
    bmi.bmiHeader.biHeight = -(img.height() as i32);
    let mut ppv_bits = ptr::null_mut();

    let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut ppv_bits, ptr::null_mut(), 0);
    let data = img.data();
    let ppv_bits = ppv_bits as *mut u8;
    for (i, px) in data.chunks_exact(4).enumerate() {
        let i = i as isize;
        let r = px[0];
        let g = px[1];
        let b = px[2];
        let a = px[3];
        ptr::write(ppv_bits.offset(i*4), b);
        ptr::write(ppv_bits.offset(i*4+1), g);
        ptr::write(ppv_bits.offset(i*4+2), r);
        ptr::write(ppv_bits.offset(i*4+3), a);
    }
    hbitmap
}
