use std::ffi::OsString;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf, is_separator};
use figdriver::Error;

mod cli;

const FONT_DIR     : &str = "/usr/share/figlet";
const DEFAULT_FONT : &str = "standard.flf";
const DEFAULT_WIDTH: usize = 80;

const LICENSE: &str = include_str!("../../../LICENSE");

const fn first_line(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut end = 0;
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    s.split_at(end).0
}

const COPYRIGHT_NOTICE: &str = first_line(LICENSE);


fn main() -> Result<(), Error> {
    let mut args = cli::Args::from_env();

    if args.contains(["-v", "--version"]) {
        println!("figlet {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.contains(["-h", "--help"]) {
        println!("Usage: figlet [options] message
  -c, --center             center the output horizontally
  -d, --dir <dir>          set the default font directory
  -f, --font <name>        specify the figfont to use
  -h, --help               display usage information and exit
  -I, --infocode <num>     print info for infocode (0-5) and exit
  -k, --kern               use kerning mode to display characters
  -l, --left               left-align the output
  -L, --left-to-right      force left-to-right print direction
  -m, --layout-mode <num>  override the font layout mode
  -n, --normal             use normal mode (each newline causes a line break)
  -o, --overlap            use character overlapping mode
  -p, --paragraph          ignore mid-paragraph line breaks
  -R, --right-to-left      enable right-to-left print direction
  -r, --right              right-align the output
  -s, --smush-default      smushing respecting font default layout mode
  -S, --smush              force smushing mode to display characters
  -v, --version            display version information and exit
  -W, --full-width         display characters in full width
  -w, --width <cols>       set the output width
  -X, --font-direction     use font file's default print direction
  -x, --default-justification
                           default justification (left for LTR, right for RTL)");
        return Ok(());
    }

    let infocode: Option<i32> = args.opt_value_from_str::<i32>(["-I", "--infocode"])
        .map_err(|e| Error::Cli(e.to_string()))?;

    let font_dir = args.opt_value_from_str::<String>(["-d", "--dir"])
        .map_err(|e| Error::Cli(e.to_string()))?
        .unwrap_or(FONT_DIR.to_string());

    let font_name = args.opt_value_from_str::<String>(["-f", "--font"])
        .map_err(|e| Error::Cli(e.to_string()))?;

    let width: usize = args.opt_value_from_str::<usize>(["-w", "--width"])
        .map_err(|e| Error::Cli(e.to_string()))?
        .unwrap_or(DEFAULT_WIDTH);

    let use_kern = args.contains(["-k", "--kern"]);
    let use_overlap = args.contains(["-o", "--overlap"]);
    let use_full_width = args.contains(["-W", "--full-width"]);
    let use_smush = args.contains(["-s", "--smush-default"]);
    let use_smush_force = args.contains(["-S", "--smush"]);

    let layout_mode: Option<i32> = args.opt_value_from_str::<i32>(["-m", "--layout-mode"])
        .map_err(|e| Error::Cli(e.to_string()))?;

    let paragraph = args.last_of(&[
        (true,  &["-p", "--paragraph"]),
        (false, &["-n", "--normal"]),
    ]).unwrap_or(false);

    let print_dir = args.last_of(&[
        (PrintDir::Ltr,         &["-L", "--left-to-right"]),
        (PrintDir::Rtl,         &["-R", "--right-to-left"]),
        (PrintDir::FontDefault, &["-X", "--font-direction"]),
    ]).unwrap_or(PrintDir::FontDefault);

    let justify = args.last_of(&[
        (Justify::Left,    &["-l", "--left"]),
        (Justify::Right,   &["-r", "--right"]),
        (Justify::Center,  &["-c", "--center"]),
        (Justify::Default, &["-x", "--default-justification"]),
    ]).unwrap_or(Justify::Default);

    if let Some(code) = infocode {
        let display_font = font_name.as_deref().unwrap_or(DEFAULT_FONT);
        print_infocode(code, &font_dir, display_font, width);
        return Ok(());
    }

    let mut fontpath = PathBuf::from(font_dir);
    if let Some(name) = font_name {
        fontpath = find_font(fontpath, name);
    } else {
        fontpath.push(DEFAULT_FONT);
    }

    let msg: String = args.finish().into_iter()
        .filter_map(|s: OsString| s.into_string().ok())
        .collect::<Vec<_>>()
        .join(" ");

    run(&fontpath, &msg, &RunConfig {
            kern: use_kern,
            overlap: use_overlap,
            full_width: use_full_width,
            print_dir,
            width,
            paragraph,
            justify,
            smush: use_smush,
            smush_force: use_smush_force,
            layout_mode,
        })
}

fn print_infocode(code: i32, font_dir: &str, font_name: &str, width: usize) {
    match code {
       0 => {
            println!("{}", COPYRIGHT_NOTICE);
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
        }
        1 => {
            print_version_int();
        }
        2 => {
            println!("{}", font_dir);
        }
        3 => {
            println!("{}", strip_font_suffix(font_name));
        }
        4 => {
            println!("{}", width);
        }
        5 => {
            println!("flf2");
        }
        // Reference figlet exits silently (code 0) for unsupported infocodes.
        _ => {}
    }
}

fn print_version_int() {
    let major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap_or(0);
    let minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap_or(0);
    let patch = env!("CARGO_PKG_VERSION_PATCH").chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u32>().unwrap_or(0);
    println!("{}", major * 10000 + minor * 100 + patch);
}

fn strip_font_suffix(name: &str) -> &str {
    name.strip_suffix(".flf")
        .or_else(|| name.strip_suffix(".tlf"))
        .unwrap_or(name)
}

fn find_font(mut fontpath: PathBuf, mut name: String) -> PathBuf {
    if !name.ends_with(".flf") && !name.ends_with(".tlf") {
        name = format!("{}.flf", name);
    }

    if name.starts_with(is_separator) {
        return PathBuf::from(name);
    }

    fontpath.push(&name);
    if fontpath.exists() {
        return fontpath;
    }

    PathBuf::from(name)
}

/// Text print direction (controlled by -L, -R, -X)
#[derive(Clone)]
enum PrintDir {
    /// Force left-to-right (-L)
    Ltr,
    /// Force right-to-left (-R)
    Rtl,
    /// Use font's default (-X, the default)
    FontDefault,
}

/// Output justification (controlled by -l, -r, -c, -x)
#[derive(Clone)]
enum Justify {
    /// Flush left (-l)
    Left,
    /// Flush right (-r)
    Right,
    /// Center (-c)
    Center,
    /// Follow print direction: left for LTR, right for RTL (-x, the default)
    Default,
}

struct RunConfig {
    kern: bool,
    overlap: bool,
    full_width: bool,
    print_dir: PrintDir,
    width: usize,
    paragraph: bool,
    justify: Justify,
    smush: bool,
    smush_force: bool,
    layout_mode: Option<i32>,
}

fn run(path: &Path, msg: &str, cfg: &RunConfig) -> Result<(), Error> {
    if !path.exists() {
        return Err(Error::FontNotFound(path.to_path_buf()));
    }

    let font = figdriver::FIGfont::from_path(path)?;
    let mut sm = figdriver::Smusher::new(&font);

    if let Some(m) = cfg.layout_mode {
        match m {
            0 => {
                sm.mode = figdriver::SMUSH_KERN;
                sm.full_width = false;
            }
            -1 => {
                sm.full_width = true;
            }
            -2 => {
                if (font.layout & figdriver::SMUSH_ENABLE) != 0 {
                    sm.mode = font.layout;
                    sm.full_width = false;
                }
            }
            1.. => {
                sm.mode = m as u32;
                sm.full_width = false;
            }
            _ => {
                return Err(Error::Cli(format!("Invalid mode value: {}", m)));
            }
        }
    } else if cfg.smush_force {
        if (font.layout & figdriver::SMUSH_ENABLE) != 0 {
            sm.mode = font.layout;
        } else {
            sm.mode = 0;
        }
        sm.full_width = false;
    } else if cfg.smush && (font.layout & figdriver::SMUSH_ENABLE) != 0 {
        sm.mode = font.layout;
        sm.full_width = false;
    } else if cfg.overlap {
        sm.mode = 0;
    } else if cfg.kern {
        sm.mode = figdriver::SMUSH_KERN;
    }

    if cfg.full_width && !cfg.smush && !cfg.smush_force {
        sm.full_width = true;
    }

    let resolved_rtl = match cfg.print_dir {
        PrintDir::Ltr => false,
        PrintDir::Rtl => true,
        PrintDir::FontDefault => sm.right2left,
    };
    sm.right2left = resolved_rtl;

    // Subtract 1 from width to match figlet's quirk: figlet treats `-w N` as
    // "allow lines up to N-1 characters" rather than N characters.
    let mut wr = figdriver::Wrapper::new(sm, cfg.width - 1);

    match cfg.justify {
        Justify::Left => {
            wr.align = figdriver::Align::Left;
        }
        Justify::Right => {
            wr.align = figdriver::Align::Right;
        }
        Justify::Center => {
            wr.align = figdriver::Align::Center;
        }
        Justify::Default => {
            wr.align = if resolved_rtl {
                figdriver::Align::Right
            } else {
                figdriver::Align::Left
            };
        }
    }

    if !msg.is_empty() {
        write_line(&mut wr, msg);
    } else {
        let input = io::BufReader::new(io::stdin());
        if cfg.paragraph {
            for line in input.lines() {
                let line = line?;
                write_paragraph(&mut wr, &line);
            }
            print_output(&wr.get());
        } else {
            for line in input.lines() {
                let line = line?;
                write_line(&mut wr, &line);
            }
        }
    }

    Ok(())
}

fn write_line(wr: &mut figdriver::Wrapper, s: &str) {
    wr.clear();
    write_tokens(wr, s);
    print_output(&wr.get());
}

fn write_paragraph(wr: &mut figdriver::Wrapper, s: &str) {
    if s.starts_with(char::is_whitespace) && !wr.is_empty() {
        print_output(&wr.get());
        wr.clear();
    }
    write_tokens(wr, s);
}

fn write_tokens(wr: &mut figdriver::Wrapper, s: &str) {
    let mut chars = s.char_indices().peekable();
    let mut start = 0;

    while let Some((_, c)) = chars.next() {
        let is_ws = c.is_whitespace();
        while let Some(&(_, next_c)) = chars.peek() {
            if next_c.is_whitespace() == is_ws && !(is_ws && next_c != ' ') {
                chars.next();
            } else {
                break;
            }
        }
        let end = chars.peek().map(|(idx, _)| *idx).unwrap_or(s.len());
        wr.wrap_str(&s[start..end], &print_output);
        start = end;
    }
}

fn print_output(v: &[String]) {
    for x in v {
        println!("{}", x);
    }
}
