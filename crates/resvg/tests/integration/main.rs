use once_cell::sync::Lazy;
use rgb::{FromSlice, RGBA8};
use usvg::{fontdb, TreeParsing, TreeTextToPath};

#[rustfmt::skip]
mod render;

mod extra;

const IMAGE_SIZE: u32 = 300;

static GLOBAL_FONTDB: Lazy<std::sync::Mutex<fontdb::Database>> = Lazy::new(|| {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir("tests/fonts");
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    std::sync::Mutex::new(fontdb)
});

pub fn render(name: &str) -> usize {
    let svg_path = format!("tests/{}.svg", name);
    let png_path = format!("tests/{}.png", name);

    let mut opt = usvg::Options::default();
    opt.resources_dir = Some(
        std::path::PathBuf::from(&svg_path)
            .parent()
            .unwrap()
            .to_owned(),
    );

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        let mut tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
        let db = GLOBAL_FONTDB.lock().unwrap();
        tree.convert_text(&db);
        tree
    };

    let rtree = resvg::Tree::from_usvg(&tree);
    let size = rtree.size.to_int_size().scale_to_width(IMAGE_SIZE).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    let render_ts = tiny_skia::Transform::from_scale(
        size.width() as f32 / tree.size.width() as f32,
        size.height() as f32 / tree.size.height() as f32,
    );
    rtree.render(render_ts, &mut pixmap.as_mut());

    // pixmap.save_png(&format!("tests/{}.png", name)).unwrap();

    let mut rgba = pixmap.take();
    demultiply_alpha(rgba.as_mut_slice().as_rgba_mut());

    let expected_data = load_png(&png_path);
    assert_eq!(expected_data.len(), rgba.len());

    let mut pixels_d = 0;
    for (a, b) in expected_data
        .as_slice()
        .as_rgba()
        .iter()
        .zip(rgba.as_rgba())
    {
        if is_pix_diff(*a, *b) {
            pixels_d += 1;
        }
    }

    // Save diff if needed.
    // if pixels_d != 0 {
    //     gen_diff(&name, &expected_data, rgba.as_slice()).unwrap();
    // }

    pixels_d
}

pub fn render_extra_with_scale(name: &str, scale: f32) -> usize {
    let svg_path = format!("tests/{}.svg", name);
    let png_path = format!("tests/{}.png", name);

    let opt = usvg::Options::default();

    let tree = {
        let svg_data = std::fs::read(&svg_path).unwrap();
        usvg::Tree::from_data(&svg_data, &opt).unwrap()
    };
    let rtree = resvg::Tree::from_usvg(&tree);

    let size = rtree.size.to_int_size().scale_by(scale).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

    let render_ts = tiny_skia::Transform::from_scale(scale, scale);
    rtree.render(render_ts, &mut pixmap.as_mut());

    // pixmap.save_png(&format!("tests/{}.png", name)).unwrap();

    let mut rgba = pixmap.take();
    demultiply_alpha(rgba.as_mut_slice().as_rgba_mut());

    let expected_data = load_png(&png_path);
    assert_eq!(expected_data.len(), rgba.len());

    let mut pixels_d = 0;
    for (a, b) in expected_data
        .as_slice()
        .as_rgba()
        .iter()
        .zip(rgba.as_rgba())
    {
        if is_pix_diff(*a, *b) {
            pixels_d += 1;
        }
    }

    // Save diff if needed.
    // if pixels_d != 0 {
    //     gen_diff(&name, &expected_data, rgba.as_slice()).unwrap();
    // }

    pixels_d
}

pub fn render_extra(name: &str) -> usize {
    render_extra_with_scale(name, 1.0)
}

fn load_png(path: &str) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let mut decoder = png::Decoder::new(data.as_slice());
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut img_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut img_data).unwrap();

    match info.color_type {
        png::ColorType::Rgb => {
            panic!("RGB PNG is not supported.");
        }
        png::ColorType::Rgba => img_data,
        png::ColorType::Grayscale => {
            let mut rgba_data = Vec::with_capacity(img_data.len() * 4);
            for gray in img_data {
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(255);
            }

            rgba_data
        }
        png::ColorType::GrayscaleAlpha => {
            let mut rgba_data = Vec::with_capacity(img_data.len() * 2);
            for slice in img_data.chunks(2) {
                let gray = slice[0];
                let alpha = slice[1];
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(alpha);
            }

            rgba_data
        }
        png::ColorType::Indexed => {
            panic!("Indexed PNG is not supported.");
        }
    }
}

// TODO: remove
fn is_pix_diff(c1: rgb::RGBA8, c2: rgb::RGBA8) -> bool {
    (c1.r as i32 - c2.r as i32).abs() > 1
        || (c1.g as i32 - c2.g as i32).abs() > 1
        || (c1.b as i32 - c2.b as i32).abs() > 1
        || (c1.a as i32 - c2.a as i32).abs() > 1
}

#[allow(dead_code)]
fn gen_diff(name: &str, img1: &[u8], img2: &[u8]) -> Result<(), png::EncodingError> {
    assert_eq!(img1.len(), img2.len());

    let mut img3 = Vec::with_capacity((img1.len() as f32 * 0.75).round() as usize);
    for (a, b) in img1.as_rgba().iter().zip(img2.as_rgba()) {
        if is_pix_diff(*a, *b) {
            img3.push(255);
            img3.push(0);
            img3.push(0);
        } else {
            img3.push(255);
            img3.push(255);
            img3.push(255);
        }
    }

    let path = std::path::PathBuf::from(format!("tests/{}-diff.png", name));
    let file = std::fs::File::create(path)?;
    let ref mut w = std::io::BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, IMAGE_SIZE, IMAGE_SIZE);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&img3)
}

/// Demultiplies provided pixels alpha.
fn demultiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 / a + 0.5) as u8;
        p.g = (p.g as f64 / a + 0.5) as u8;
        p.r = (p.r as f64 / a + 0.5) as u8;
    }
}
