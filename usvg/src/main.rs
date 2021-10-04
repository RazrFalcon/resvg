// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use pico_args::Arguments;


const HELP: &str = "\
usvg (micro SVG) is an SVG simplification tool.

USAGE:
  usvg [OPTIONS] <in-svg> <out-svg>  # from file to file
  usvg [OPTIONS] -c <in-svg>         # from file to stdout
  usvg [OPTIONS] - <out-svg>         # from stdin to file
  usvg [OPTIONS] - -c                # from stdin to stdout

OPTIONS:
  -h, --help                    Prints help information
  -V, --version                 Prints version information
  -c                            Prints the output SVG to the stdout

  --dpi DPI                     Sets the resolution
                                [default: 96] [possible values: 10..4000]
  --languages LANG              Sets a comma-separated list of languages that
                                will be used during the 'systemLanguage'
                                attribute resolving
                                Examples: 'en-US', 'en-US, ru-RU', 'en, ru'
                                [default: en]
  --shape-rendering HINT        Selects the default shape rendering method
                                [default: geometricPrecision]
                                [possible values: optimizeSpeed, crispEdges,
                                geometricPrecision]
  --text-rendering HINT         Selects the default text rendering method
                                [default: optimizeLegibility]
                                [possible values: optimizeSpeed, optimizeLegibility,
                                geometricPrecision]
  --image-rendering HINT        Selects the default image rendering method
                                [default: optimizeQuality]
                                [possible values: optimizeQuality, optimizeSpeed]
  --resources-dir DIR           Sets a directory that will be used during
                                relative paths resolving.
                                Expected to be the same as the directory that
                                contains the SVG file, but can be set to any.
                                [default: input file directory
                                or none when reading from stdin]

  --font-family FAMILY          Sets the default font family that will be
                                used when no 'font-family' is present
                                [default: Times New Roman]
  --font-size SIZE              Sets the default font size that will be
                                used when no 'font-size' is present
                                [default: 12] [possible values: 1..192]
  --serif-family FAMILY         Sets the 'serif' font family
                                [default: Times New Roman]
  --sans-serif-family FAMILY    Sets the 'sans-serif' font family
                                [default: Arial]
  --cursive-family FAMILY       Sets the 'cursive' font family
                                [default: Comic Sans MS]
  --fantasy-family FAMILY       Sets the 'fantasy' font family
                                [default: Impact]
  --monospace-family FAMILY     Sets the 'monospace' font family
                                [default: Courier New]
  --use-font-file PATH          Load a specified font file into the fonts database.
                                Will be used during text to path conversion.
                                This option can be set multiple times
  --use-fonts-dir PATH          Loads all fonts from the specified directory
                                into the fonts database.
                                Will be used during text to path conversion.
                                This option can be set multiple times
  --skip-system-fonts           Disables system fonts loading.
                                You should add some fonts manually using
                                --use-font-file and/or --use-fonts-dir
                                Otherwise, text elements will not be processes
  --list-fonts                  Lists successfully loaded font faces.
                                Useful for debugging
  --default-width LENGTH        Sets the default width of the SVG viewport. Like
                                the '--default-height' option, this option
                                controlls what size relative units in the document
                                will use as a base if there is no viewBox and
                                document width or height are relative.
                                [values: 1..2^32] [default: 100]
  --default-height LENGTH       Sets the default height of the SVG viewport.
                                Refer to the explanation of the '--default-width'
                                option. [values: 1..2^32] [default: 100]

  --keep-named-groups           Disables removing of groups with non-empty ID
  --id-prefix                   Adds a prefix to each ID attribute
  --indent INDENT               Sets the XML nodes indent
                                [values: none, 0, 1, 2, 3, 4, tabs] [default: 4]
  --attrs-indent INDENT         Sets the XML attributes indent
                                [values: none, 0, 1, 2, 3, 4, tabs] [default: none]

  --quiet                       Disables warnings

ARGS:
  <in-svg>                      Input file
  <out-svg>                     Output file
";

#[derive(Debug)]
struct Args {
    dpi: u32,
    languages: Vec<String>,
    shape_rendering: usvg::ShapeRendering,
    text_rendering: usvg::TextRendering,
    image_rendering: usvg::ImageRendering,
    resources_dir: Option<PathBuf>,

    font_family: Option<String>,
    font_size: u32,
    serif_family: Option<String>,
    sans_serif_family: Option<String>,
    cursive_family: Option<String>,
    fantasy_family: Option<String>,
    monospace_family: Option<String>,
    font_files: Vec<PathBuf>,
    font_dirs: Vec<PathBuf>,
    skip_system_fonts: bool,
    list_fonts: bool,
    default_width: u32,
    default_height: u32,

    keep_named_groups: bool,
    id_prefix: Option<String>,
    indent: xmlwriter::Indent,
    attrs_indent: xmlwriter::Indent,

    quiet: bool,

    input: String,
    output: String,
}

fn collect_args() -> Result<Args, pico_args::Error> {
    let mut input = Arguments::from_env();

    if input.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    if input.contains(["-V", "--version"]) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    Ok(Args {
        dpi:                input.opt_value_from_fn("--dpi", parse_dpi)?.unwrap_or(96),
        languages:          input.opt_value_from_fn("--languages", parse_languages)?
                                 .unwrap_or(vec!["en".to_string()]), // TODO: use system language
        shape_rendering:    input.opt_value_from_str("--shape-rendering")?.unwrap_or_default(),
        text_rendering:     input.opt_value_from_str("--text-rendering")?.unwrap_or_default(),
        image_rendering:    input.opt_value_from_str("--image-rendering")?.unwrap_or_default(),
        resources_dir:      input.opt_value_from_str("--resources-dir").unwrap_or_default(),

        font_family:        input.opt_value_from_str("--font-family")?,
        font_size:          input.opt_value_from_fn("--font-size", parse_font_size)?.unwrap_or(12),
        serif_family:       input.opt_value_from_str("--serif-family")?,
        sans_serif_family:  input.opt_value_from_str("--sans-serif-family")?,
        cursive_family:     input.opt_value_from_str("--cursive-family")?,
        fantasy_family:     input.opt_value_from_str("--fantasy-family")?,
        monospace_family:   input.opt_value_from_str("--monospace-family")?,
        font_files:         input.values_from_str("--use-font-file")?,
        font_dirs:          input.values_from_str("--use-fonts-dir")?,
        skip_system_fonts:  input.contains("--skip-system-fonts"),
        list_fonts:         input.contains("--list-fonts"),
        default_width:      input.opt_value_from_fn("--default-width", parse_length)?.unwrap_or(100),
        default_height:     input.opt_value_from_fn("--default-height", parse_length)?.unwrap_or(100),

        keep_named_groups:  input.contains("--keep-named-groups"),
        id_prefix:          input.opt_value_from_str("--id-prefix")?,
        indent:             input.opt_value_from_fn("--indent", parse_indent)?
                                 .unwrap_or(xmlwriter::Indent::Spaces(4)),
        attrs_indent:       input.opt_value_from_fn("--attrs-indent", parse_indent)?
                                 .unwrap_or(xmlwriter::Indent::None),

        quiet:              input.contains("--quiet"),

        input:              input.free_from_str()?,
        output:             input.free_from_str()?,
    })
}

fn parse_dpi(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n >= 10 && n <= 4000 {
        Ok(n)
    } else {
        Err("DPI out of bounds".to_string())
    }
}

fn parse_font_size(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n > 0 && n <= 192 {
        Ok(n)
    } else {
        Err("font size out of bounds".to_string())
    }
}

fn parse_languages(s: &str) -> Result<Vec<String>, String> {
    let mut langs = Vec::new();
    for lang in s.split(',') {
        langs.push(lang.trim().to_string());
    }

    if langs.is_empty() {
        return Err("languages list cannot be empty".to_string());
    }

    Ok(langs)
}

fn parse_indent(s: &str) -> Result<xmlwriter::Indent, String> {
    let indent = match s {
        "none" => xmlwriter::Indent::None,
        "0" => xmlwriter::Indent::Spaces(0),
        "1" => xmlwriter::Indent::Spaces(1),
        "2" => xmlwriter::Indent::Spaces(2),
        "3" => xmlwriter::Indent::Spaces(3),
        "4" => xmlwriter::Indent::Spaces(4),
        "tabs" => xmlwriter::Indent::Tabs,
        _ => return Err("invalid INDENT value".to_string()),
    };

    Ok(indent)
}

fn parse_length(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid length")?;

    if n > 0 {
        Ok(n)
    } else {
        Err("LENGTH cannot be zero".to_string())
    }
}

#[derive(Clone, PartialEq, Debug)]
enum InputFrom<'a> {
    Stdin,
    File(&'a str),
}

#[derive(Clone, PartialEq, Debug)]
enum OutputTo<'a> {
    Stdout,
    File(&'a str),
}


fn main() {
    let args = match collect_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            process::exit(1);
        }
    };

    if !args.quiet {
        if let Ok(()) = log::set_logger(&LOGGER) {
            log::set_max_level(log::LevelFilter::Warn);
        }
    }

    if let Err(e) = process(args) {
        eprintln!("Error: {}.", e.to_string());
        std::process::exit(1);
    }
}

fn process(args: Args) -> Result<(), String> {
    let (in_svg, out_svg) = {
        let in_svg = args.input.as_str();
        let out_svg = args.output.as_str();

        let svg_from = if in_svg == "-" {
            InputFrom::Stdin
        } else if in_svg == "-c" {
            return Err(format!("-c should be set after input"));
        } else {
            InputFrom::File(in_svg)
        };

        let svg_to = if out_svg == "-c" {
            OutputTo::Stdout
        } else {
            OutputTo::File(out_svg)
        };

        (svg_from, svg_to)
    };

    let mut fontdb = usvg::fontdb::Database::new();
    if !args.skip_system_fonts {
        fontdb.load_system_fonts();
    }

    for path in &args.font_files {
        if let Err(e) = fontdb.load_font_file(path) {
            log::warn!("Failed to load '{}' cause {}.", path.display(), e);
        }
    }

    for path in &args.font_dirs {
        fontdb.load_fonts_dir(path);
    }

    let take_or = |mut family: Option<String>, fallback: &str|
        family.take().unwrap_or_else(|| fallback.to_string());

    fontdb.set_serif_family(take_or(args.serif_family, "Times New Roman"));
    fontdb.set_sans_serif_family(take_or(args.sans_serif_family, "Arial"));
    fontdb.set_cursive_family(take_or(args.cursive_family, "Comic Sans MS"));
    fontdb.set_fantasy_family(take_or(args.fantasy_family, "Impact"));
    fontdb.set_monospace_family(take_or(args.monospace_family, "Courier New"));

    if args.list_fonts {
        for face in fontdb.faces() {
            if let usvg::fontdb::Source::File(ref path) = &face.source {
                println!(
                    "{}: '{}', {}, {:?}, {:?}, {:?}",
                    path.display(), face.family, face.index,
                    face.style, face.weight.0, face.stretch
                );
            }
        }
    }

    let resources_dir = match args.resources_dir {
        Some(v) => Some(v),
        None => {
            match in_svg {
                InputFrom::Stdin => None,
                InputFrom::File(ref f) => {
                    // Get input file absolute directory.
                    std::fs::canonicalize(f).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
                }
            }
        }
    };

    let re_opt = usvg::Options {
        resources_dir,
        dpi: args.dpi as f64,
        font_family: take_or(args.font_family, "Times New Roman"),
        font_size: args.font_size as f64,
        languages: args.languages,
        shape_rendering: args.shape_rendering,
        text_rendering: args.text_rendering,
        image_rendering: args.image_rendering,
        keep_named_groups: args.keep_named_groups,
        default_size: usvg::Size::new(args.default_width as f64, args.default_height as f64).unwrap(),
        fontdb,
    };

    let input_svg = match in_svg {
        InputFrom::Stdin => load_stdin(),
        InputFrom::File(ref path) => std::fs::read(path).map_err(|e| e.to_string()),
    }?;

    let tree = usvg::Tree::from_data(&input_svg, &re_opt.to_ref()).map_err(|e| format!("{}", e))?;

    let xml_opt = usvg::XmlOptions {
        id_prefix: args.id_prefix,
        writer_opts: xmlwriter::Options {
            use_single_quote: false,
            indent: args.indent,
            attributes_indent: args.attrs_indent,
        }
    };

    let s = tree.to_string(&xml_opt);
    match out_svg {
        OutputTo::Stdout => {
            io::stdout()
                .write_all(s.as_bytes())
                .map_err(|_| format!("failed to write to the stdout"))?;
        }
        OutputTo::File(path) => {
            let mut f = File::create(path)
                .map_err(|_| format!("failed to create the output file"))?;
            f.write_all(s.as_bytes())
                .map_err(|_| format!("failed to write to the output file"))?;
        }
    }

    Ok(())
}

fn load_stdin() -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle
        .read_to_end(&mut buf)
        .map_err(|_| format!("failed to read from stdin"))?;

    Ok(buf)
}


/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if record.target().len() > 0 {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, record.args()),
                log::Level::Warn  => eprintln!("Warning (in {}:{}): {}", target, line, record.args()),
                log::Level::Info  => eprintln!("Info (in {}:{}): {}", target, line, record.args()),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, record.args()),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, record.args()),
            }
        }
    }

    fn flush(&self) {}
}
