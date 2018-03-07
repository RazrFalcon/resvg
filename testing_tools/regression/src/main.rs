extern crate clap;
extern crate git2 as git;
extern crate walkdir;
extern crate image;
extern crate subprocess;

use std::fmt;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use image::GenericImage;
use walkdir::{WalkDir, WalkDirIterator};
use clap::{Arg, App};
use subprocess::Exec;

macro_rules! dir_iter {
    ($input_dir:expr) => (
        WalkDir::new($input_dir).into_iter().filter_entry(|x| is_svg(x)).map(|x| x.unwrap())
    )
}

#[derive(Debug)]
enum Error {
    Different,
    RenderError,
    RenderCrashed,
    SizeMismatch((u32, u32), (u32, u32)),
    Io(io::Error),
    Git(git::Error),
    Image(image::ImageError),
    Subprocess(subprocess::PopenError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Different => {
                write!(f, "Rendered images are different")
            }
            Error::RenderError => {
                write!(f, "Render exited with an error")
            }
            Error::RenderCrashed => {
                write!(f, "Current render crashed")
            }
            Error::SizeMismatch((w1, h1), (w2, h2)) => {
                write!(f, "Image size mismatch: {}x{} vs {}x{}", w1, h1, w2, h2)
            }
            Error::Io(ref e) => {
                write!(f, "'io' error: {:?}", e)
            }
            Error::Git(ref e) => {
                write!(f, "'git' error: {:?}", e)
            }
            Error::Image(ref e) => {
                write!(f, "'image' error: {:?}", e)
            }
            Error::Subprocess(ref e) => {
                write!(f, "'subprocess' error: {:?}", e)
            }
        }
    }
}

macro_rules! from_error {
    ($err_name:ident, $err_type:ty) => (
        impl From<$err_type> for Error {
            fn from(value: $err_type) -> Error {
                Error::$err_name(value)
            }
        }
    )
}

from_error!(Io, io::Error);
from_error!(Git, git::Error);
from_error!(Image, image::ImageError);
from_error!(Subprocess, subprocess::PopenError);

type Result<T> = std::result::Result<T, Error>;

struct Data<'a> {
    work_dir: &'a Path,
    input_dir: &'a Path,
    curr_render: &'a Path,
    prev_render: Option<PathBuf>,
    is_use_prev_commit: bool,
    backend: &'a str,
    allowed_files: Vec<String>,
}

fn main() {
    let m = App::new("files-testing")
        .arg(Arg::with_name("workdir")
            .long("workdir").help("Sets path to the work dir")
            .value_name("DIR")
            .required(true))
        .arg(Arg::with_name("backend")
            .long("backend").help("Sets resvg backend")
            .takes_value(true)
            .possible_values(&["qt", "cairo"])
            .required(true))
        .arg(Arg::with_name("use-prev-commit")
            .long("use-prev-commit").help("Use previous git commit as a reference"))
        .get_matches();

    let render = Path::new("../../tools/rendersvg/target/debug/rendersvg");
    if !render.exists() {
        println!("Error: {:?} not found.", render);
        return;
    }

    let backend = m.value_of("backend").unwrap();

    let data = Data {
        work_dir: Path::new(m.value_of("workdir").unwrap()),
        input_dir: Path::new("../images/svg"),
        prev_render: None,
        curr_render: render,
        is_use_prev_commit: m.is_present("use-prev-commit"),
        backend: backend,
        allowed_files: load_allowed_file_list(backend),
    };

    if let Err(e) = process(data) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

// TODO: skip comments
fn load_allowed_file_list(backend: &str) -> Vec<String> {
    if let Ok(f) = fs::File::open(&format!("allow-{}.txt", backend)) {
        return io::BufReader::new(&f).lines().map(|l| l.unwrap()).collect();
    }

    Vec::new()
}

fn process(mut data: Data) -> Result<()> {
    // Create work dir.
    if !data.work_dir.exists() {
        fs::create_dir(&data.work_dir)?;
    }

    // Clone and build latest master.
    data.prev_render = Some(build_prev_version(&data)?);

    run_tests(&data)?;

    Ok(())
}

fn build_prev_version(data: &Data) -> Result<PathBuf> {
    let url = "https://github.com/RazrFalcon/resvg";
    let repo_path = Path::new(data.work_dir).join("resvg");

    let repo = if repo_path.exists() {
        let repo = git::Repository::open(&repo_path)?;
        repo.find_remote("origin")?.fetch(&["master"], None, None)?;

        repo
    } else {
        git::Repository::clone(url, &repo_path)?
    };


    if data.is_use_prev_commit {
        let prev_oid = {
            let mut rw = repo.revwalk()?;
            rw.push_head()?;
            rw.next();
            rw.next().unwrap().unwrap()
        };

        let prev_obj = repo.find_object(prev_oid, None)?;
        repo.checkout_tree(&prev_obj, Some(git::build::CheckoutBuilder::new().force()))?;

        println!("checkout: {:?}", prev_oid);
    }


    Exec::cmd("cargo")
        .cwd(repo_path.join("tools/rendersvg"))
        .arg("build")
        .arg("--features")
        .arg(data.backend.to_owned() + "-backend")
        .join()?;

    let bin_path = repo_path.join("tools/rendersvg/target/debug/rendersvg");

    Ok(bin_path)
}

fn run_tests(data: &Data) -> Result<()> {
    let mut files = Vec::new();
    for entry in dir_iter!(data.input_dir) {
        if entry.file_type().is_file() {
            files.push(entry.path().to_owned());
        }
    }

    files.sort();

    let mut idx = 1;
    for file_path in &files {
        let sub_path: String = file_path.strip_prefix(data.input_dir).unwrap()
                                        .to_str().unwrap().into();

        println!("Test {} of {}: {}", idx, files.len(), sub_path);

        run_test(data, &file_path)?;

        idx += 1;
    }

    Ok(())
}

fn run_test(data: &Data, svg_path: &Path) -> Result<()> {
    let png_path_prev = svg_to_png(data.work_dir, svg_path, "_prev");
    let png_path_curr = svg_to_png(data.work_dir, svg_path, "_curr");

    // We are using original svg path for rendering, because of relative images.
    let prev_render = data.prev_render.as_ref().unwrap();
    if let Err(e) = render(prev_render, svg_path, &png_path_prev, data.backend) {
        match e {
              Error::RenderError
            | Error::RenderCrashed => {
                // Previous version can crash. We don't care.
                return Ok(());
            }
            _ => return Err(e),
        }
    }

    render(data.curr_render, svg_path, &png_path_curr, data.backend)?;

    let diff_path = svg_to_png(data.work_dir, svg_path, "_diff");

    let diff = compare_imgs(&png_path_prev, &png_path_curr, &diff_path)?;

    let sub_path: String = svg_path.strip_prefix(data.input_dir).unwrap()
                                   .to_str().unwrap().into();

    if !data.allowed_files.contains(&sub_path) {
        if diff > 20 { // No need to be that strict.
            return Err(Error::Different);
        }
    }

    fs::remove_file(png_path_curr)?;
    fs::remove_file(png_path_prev)?;

    if diff_path.exists() {
        fs::remove_file(diff_path)?;
    }

    Ok(())
}

fn svg_to_png(workdir: &Path, path: &Path, suffix: &str) -> PathBuf {
    let file_stem = path.file_stem().unwrap().to_str().unwrap().to_owned() + suffix;
    PathBuf::from(workdir).join(file_stem).with_extension("png")
}

fn render(exe: &Path, in_svg: &Path, out_png: &Path, backend: &str) -> Result<()> {
    let o = Exec::cmd(exe)
        .arg(in_svg)
        .arg(out_png)
        .arg("--backend")
        .arg(backend)
        .capture()?;

    if o.exit_status == subprocess::ExitStatus::Exited(1) {
        return Err(Error::RenderError);
    }

    if o.exit_status == subprocess::ExitStatus::Exited(101) {
        return Err(Error::RenderCrashed);
    }

    Ok(())
}

fn compare_imgs(path1: &Path, path2: &Path, path_diff: &Path) -> Result<u32> {
    let img1 = image::open(&path1)?;
    let img2 = image::open(&path2)?;

    if img1.dimensions() != img2.dimensions() {
        return Err(Error::SizeMismatch(img1.dimensions(), img2.dimensions()));
    }

    let px1 = img1.pixels();
    let px2 = img2.pixels();

    let (w, h) = img1.dimensions();
    let mut diff_img = image::ImageBuffer::new(w, h);

    let red = image::Rgb([255, 0, 0]);
    let white = image::Rgb([255, 255, 255]);

    let mut count = 0;
    for (p1, p2) in px1.zip(px2) {
        if p1 != p2 {
            diff_img.put_pixel(p1.0, p1.1, red);
            count += 1;
        } else {
            diff_img.put_pixel(p1.0, p1.1, white);
        }
    }

    if count != 0 {
        let f = &mut fs::File::create(path_diff)?;
        image::ImageRgb8(diff_img).save(f, image::PNG)?;
    }

    Ok(count)
}

fn is_svg(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_file() {
        match entry.path().extension() {
            Some(e) => {
                let s = e.to_str().unwrap();
                s == "svg" || s == "svgz"
            }
            None => false,
        }
    } else {
        true
    }
}
