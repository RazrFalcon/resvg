use std::fmt;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::Command;

use wait_timeout::ChildExt;

use rayon::prelude::*;

const RESVG_URL: &str = "https://github.com/RazrFalcon/resvg";

// List of files that should be skipped.
const CRASH_ALLOWED: &[&str] = &[
    "e-svg-007.svg",
    "e-svg-036.svg",
    "e-feMorphology-012.svg", // will timeout on CI
];


#[derive(Debug)]
enum ErrorKind {
    CurrRenderFailed(io::Error),
    DifferentImageSizes,
    DifferentImages(usize),
}

#[derive(Debug)]
struct Error {
    kind: ErrorKind,
    svg_file: PathBuf,
}

impl Error {
    fn new(kind: ErrorKind, svg_file: &Path) -> Self {
        Error { kind, svg_file: svg_file.to_path_buf() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_name = self.svg_file.file_name_str();
        match self.kind {
            ErrorKind::CurrRenderFailed(ref e) => {
                write!(f, "{} rendering failed cause {}", file_name, e)
            }
            ErrorKind::DifferentImageSizes => {
                write!(f, "{} was rendered with different sizes", file_name)
            }
            ErrorKind::DifferentImages(n) => {
                write!(f, "{} is different by {} pixels", file_name, n)
            }
        }
    }
}

struct Args {
    backend: String,
    use_prev_commit: bool,
    in_dir: PathBuf,
    work_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;

    // Build current version.
    Command::new("cargo")
        .args(&["build", "--release", "--features", &format!("{}-backend", args.backend)])
        .current_dir("../../tools/rendersvg")
        .run()?;

    let curr_rendersvg = fs::canonicalize("../../target/release/rendersvg")?;
    let prev_rendersvg = build_previous_version(&args)?;

    let files = collect_files(&args)?;
    let errors: Vec<_> = files.into_par_iter().filter_map(|svg_path| {
        match process_file2(&args, &curr_rendersvg, &prev_rendersvg, &svg_path) {
            Ok(_) => None,
            Err(e) => Some(e),
        }
    }).collect();

    if !errors.is_empty() {
        for e in errors {
            println!("Failed: {}.", e);
        }

        std::process::exit(1);
    }

    Ok(())
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut args = pico_args::Arguments::from_env();
    Ok(Args {
        backend: args.value_from_str("--backend")?.ok_or("backend is not set")?,
        use_prev_commit: args.contains("--use-prev-commit"),
        in_dir: args.free_from_str()?.ok_or("input dir is not set")?,
        work_dir: args.free_from_str()?.ok_or("work dir is not set")?,
    })
}

fn build_previous_version(args: &Args) -> io::Result<PathBuf> {
    let prev_resvg_dir = args.work_dir.join("resvg");

    if prev_resvg_dir.exists() {
        return Ok(prev_resvg_dir.join("target/release/rendersvg"));
    }

    Command::new("git")
        .args(&["clone", "--depth", "5", RESVG_URL, prev_resvg_dir.to_str().unwrap()])
        .run()?;

    if args.use_prev_commit {
        // TODO: maybe there is a better way
        Command::new("git")
            .args(&["reset", "--hard", "HEAD~1"])
            .current_dir(&prev_resvg_dir)
            .run()?;
    }

    Command::new("cargo")
        .args(&["build", "--release", "--features", &format!("{}-backend", args.backend)])
        .current_dir(prev_resvg_dir.join("tools/rendersvg"))
        .run()?;

    Ok(prev_resvg_dir.join("target/release/rendersvg"))
}

fn parse_allowed(backend: &str) -> io::Result<Vec<String>> {
    let mut allowed_files: Vec<_> = CRASH_ALLOWED.iter().map(|s| s.to_string()).collect();

    let file = fs::File::open(format!("allow-{}.txt", backend))?;
    for line in io::BufReader::new(file).lines() {
        allowed_files.push(line?);
    }

    Ok(allowed_files)
}

fn collect_files(args: &Args) -> io::Result<Vec<PathBuf>> {
    assert!(args.in_dir.is_dir());

    let allowed_files = parse_allowed(&args.backend)?;

    let mut files = Vec::new();

    for entry in fs::read_dir(&args.in_dir)? {
        let path = entry?.path();
        if path.is_file() {
            if allowed_files.iter().any(|s| s == path.as_path().file_name_str()) {
                continue;
            }

            files.push(path);
        }
    }

    files.sort();

    Ok(files)
}

fn change_ext(mut path: PathBuf, suffix: &str, ext: &str) -> PathBuf {
    let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
    path.set_file_name(format!("{}-{}.{}", stem, suffix, ext));
    path
}

fn render_svg(
    word_dir: &Path,
    render: &Path,
    backend: &str,
    in_svg: &Path,
    out_png: &Path,
) -> io::Result<()> {
    // Render with zoom by default to test scaling.
    // Images may render differently depending on scale.
    Command::new(render)
        .args(&[
            "--backend", backend,
            "--zoom", "2",
            in_svg.to_str().unwrap(), out_png.to_str().unwrap(),
        ])
        .current_dir(word_dir)
        .stderr(std::process::Stdio::piped())
        .run_with_timeout(15)
}

fn process_file2(
    args: &Args,
    curr_rendersvg: &Path,
    prev_rendersvg: &Path,
    svg_path: &Path,
) -> Result<(), Error> {
    let file = svg_path.file_name_str();
    let curr_png = change_ext(args.work_dir.join(file), "curr", "png");
    let prev_png = change_ext(args.work_dir.join(file), "prev", "png");

    // remove leftovers
    let _ = fs::remove_file(&curr_png);
    let _ = fs::remove_file(&prev_png);

    process_file(
        &args,
        &curr_rendersvg,
        &prev_rendersvg,
        svg_path,
        &curr_png,
        &prev_png,
    )?;

    // remove temp files
    let _ = fs::remove_file(&curr_png);
    let _ = fs::remove_file(&prev_png);

    Ok(())
}

fn process_file(
    args: &Args,
    curr_rendersvg: &Path,
    prev_rendersvg: &Path,
    in_svg: &Path,
    curr_png: &Path,
    prev_png: &Path,
) -> Result<(), Error> {
    if render_svg(&args.work_dir, prev_rendersvg, &args.backend, in_svg, prev_png).is_err() {
        return Ok(());
    }

    if let Err(e) = render_svg(&args.work_dir, curr_rendersvg, &args.backend, in_svg, curr_png) {
        return Err(Error {
            kind: ErrorKind::CurrRenderFailed(e),
            svg_file: in_svg.to_path_buf(),
        });
    }

    let prev_image = image::open(prev_png).unwrap().to_rgba();
    let curr_image = image::open(curr_png).unwrap().to_rgba();

    if curr_image.dimensions() != prev_image.dimensions() {
        return Err(Error::new(ErrorKind::DifferentImageSizes, in_svg));
    }

    if *curr_image == *prev_image {
        return Ok(());
    }

    let mut diff = 0;
    for (p1, p2) in curr_image.pixels().zip(prev_image.pixels()) {
        if p1 != p2 {
            diff += 1;
        }
    }

    if diff != 0 {
        return Err(Error::new(ErrorKind::DifferentImages(diff), in_svg));
    }

    Ok(())
}


trait PathExt {
    fn file_name_str(&self) -> &str;
}

impl PathExt for Path {
    fn file_name_str(&self) -> &str {
        self.file_name().unwrap().to_str().unwrap()
    }
}


trait CommandExt {
    fn run(&mut self) -> io::Result<()>;
    fn run_with_timeout(&mut self, sec: u64) -> io::Result<()>;
}

impl CommandExt for Command {
    fn run(&mut self) -> io::Result<()> {
        if self.status()?.success() {
            Ok(())
        } else {
            // The actual error doesn't matter.
            Err(io::ErrorKind::Other.into())
        }
    }

    fn run_with_timeout(&mut self, sec: u64) -> io::Result<()> {
        let mut child = self.spawn()?;

        let timeout = std::time::Duration::from_secs(sec);
        let status_code = match child.wait_timeout(timeout)? {
            Some(status) => status.code(),
            None => {
                child.kill()?;
                child.wait()?;
                return Err(io::ErrorKind::TimedOut.into());
            }
        };

        if status_code == Some(0) {
            Ok(())
        } else {
            let mut s = String::new();
            if let Some(mut stderr) = child.stderr {
                use std::io::Read;
                stderr.read_to_string(&mut s).unwrap();
            }

            Err(io::Error::new(io::ErrorKind::Other, s))
        }
    }
}
