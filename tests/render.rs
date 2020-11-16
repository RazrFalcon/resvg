use std::path::{Path, PathBuf};
use rgb::FromSlice;
use rayon::prelude::*;

const IMAGE_SIZE: u32 = 300;

#[test]
fn render() {
    let mut files = Vec::new();
    for entry in std::fs::read_dir("tests/svg").unwrap() {
        let path = entry.unwrap().path();
        files.push(path);
    }
    files.sort();

    let mut opt = usvg::Options::default();
    opt.font_family = "Noto Sans".to_string();
    opt.fontdb.load_fonts_dir("tests/fonts");
    opt.fontdb.set_serif_family("Noto Serif");
    opt.fontdb.set_sans_serif_family("Noto Sans");
    opt.fontdb.set_cursive_family("Yellowtail");
    opt.fontdb.set_fantasy_family("Sedgwick Ave Display");
    opt.fontdb.set_monospace_family("Noto Mono");
    opt.path = Some(files[0].clone()); // TODO: fix

    let ignore = &[
        "e-feMorphology-012", // will timeout on CI
        "e-svg-007", // invalid encoding
        "e-svg-034", // invalid size
        "e-svg-035", // invalid size
        "e-svg-036", // invalid size

        "e-svg-009",
        "e-svg-010",
        "e-svg-011",
        "e-svg-015",
        "e-svg-017",
    ];

    let results: Vec<_> = files.into_par_iter().map(|svg_path| {
        let file_name = svg_path.file_stem().unwrap().to_str().unwrap().to_owned();

        if ignore.contains(&file_name.as_str()) {
            return Ok(());
        }

        let png_path = PathBuf::from(format!("tests/png/{}.png", file_name));

        process(&file_name, &svg_path, &png_path, &opt)
            .map_err(|e| Error::new(e, &file_name))
    }).collect();

    let mut has_errors = false;
    for res in results {
        if let Err(e) = res {
            eprintln!("{}", e);
            has_errors = true;
        }
    }

    if has_errors {
        panic!("failed");
    }
}

#[derive(Debug)]
enum ErrorKind {
    Panicked,
    ParsingFailed(usvg::Error),
    RenderingFailed,
    DifferentImageSizes,
    DifferentImages(usize),
    PngEncodingError(png::EncodingError),
}

impl From<usvg::Error> for ErrorKind {
    fn from(e: usvg::Error) -> Self {
        ErrorKind::ParsingFailed(e)
    }
}

impl From<png::EncodingError> for ErrorKind {
    fn from(e: png::EncodingError) -> Self {
        ErrorKind::PngEncodingError(e)
    }
}


#[derive(Debug)]
struct Error {
    kind: ErrorKind,
    file_name: String,
}

impl Error {
    fn new(kind: ErrorKind, file_name: &str) -> Self {
        Error { kind, file_name: file_name.to_owned() }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::Panicked => {
                write!(f, "{} panicked", self.file_name)
            }
            ErrorKind::ParsingFailed(ref e) => {
                write!(f, "{} parsing failed cause {}", self.file_name, e)
            }
            ErrorKind::RenderingFailed => {
                write!(f, "{} rendering parsing", self.file_name)
            }
            ErrorKind::DifferentImageSizes => {
                write!(f, "{} was rendered with different sizes", self.file_name)
            }
            ErrorKind::DifferentImages(n) => {
                write!(f, "{} is different by {} pixels", self.file_name, n)
            }
            ErrorKind::PngEncodingError(ref e) => {
                write!(f, "PNG encoding failed at {} cause {}", self.file_name, e)
            }
        }
    }
}

fn process(
    #[allow(unused_variables)] file_name: &str,
    svg_path: &Path,
    png_path: &Path,
    opt: &usvg::Options,
) -> Result<(), ErrorKind> {
    let img = std::panic::catch_unwind(move || {
        let tree = usvg::Tree::from_file(&svg_path, &opt).map_err(|e| ErrorKind::ParsingFailed(e))?;
        resvg::render(&tree, usvg::FitTo::Width(IMAGE_SIZE), None).ok_or_else(|| ErrorKind::RenderingFailed)
    }).map_err(|_| ErrorKind::Panicked)??;

    let expected_data = load_png(png_path);
    if expected_data.len() != img.data().len() {
        return Err(ErrorKind::DifferentImageSizes);
    }

    let mut pixels_d = 0;
    for (a, b) in expected_data.as_slice().as_rgba().iter().zip(img.data().as_rgba()) {
        // Sadly, Skia can produce slightly different results on different OS'es/hardware.
        // Not sure why. Because of that, we have to use approximate comparison.
        if is_pix_diff(*a, *b) {
            pixels_d += 1;
        }
    }

    if pixels_d != 0 {
        // Save diff if needed.
        // img.save_png(&format!("tests/{}.png", file_name))?;
        // gen_diff(&file_name, &expected_data, img.data())?;
        return Err(ErrorKind::DifferentImages(pixels_d));
    }

    Ok(())
}

fn load_png(path: &Path) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let decoder = png::Decoder::new(data.as_slice());
    let (info, mut reader) = decoder.read_info().unwrap();

    let mut img_data = vec![0; info.buffer_size()];
    reader.next_frame(&mut img_data).unwrap();

    match info.color_type {
        png::ColorType::RGB => {
            panic!("RGB PNG is not supported.");
        }
        png::ColorType::RGBA => {
            img_data
        }
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

    let path = PathBuf::from(format!("tests/{}-diff.png", name));
    let file = std::fs::File::create(path)?;
    let ref mut w = std::io::BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, IMAGE_SIZE, IMAGE_SIZE);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&img3)
}

// A pixel can have a slightly different channel color due to rounding, I guess.
fn is_pix_diff(c1: rgb::RGBA8, c2: rgb::RGBA8) -> bool {
    (c1.r as i32 - c2.r as i32).abs() > 1 ||
    (c1.g as i32 - c2.g as i32).abs() > 1 ||
    (c1.b as i32 - c2.b as i32).abs() > 1 ||
    (c1.a as i32 - c2.a as i32).abs() > 1
}
