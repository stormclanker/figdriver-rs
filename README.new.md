<div aria-label="Hello" role="img">
<pre style="font-family: monospace; line-height: 1.0;">

      ___ __          __      __                                    
    .'  _|__.-----.--|  .----|__.--.--.-----.----.______.----.-----.
    |   _|  |  _  |  _  |   _|  |  |  |  -__|   _|______|   _|__ --|
    |__| |__|___  |_____|__| |__|\___/|_____|__|        |__| |_____|
            |_____|                                                 

</pre></div>

# Figriver-rs: A FIGfont renderer written in Rust

FIGdrivers render text as large ASCII-art banners using FIGfont (`.flf`) files. This
project implements the FIGfont specification to be used as a library or as a
drop-in replacement for the classic `figlet` command.

# Using the library

To render a string in a flf2 font, sent it to `Smusher`:

    use figdriver::{FIGfont, Smusher};
    
    fn run() -> Result<Vec<String>, figdriver::Error> {
        let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/standard.flf";
        let font = FIGfont::from_path(&path)?;
        let mut sm = Smusher::new(&font);
        sm.push_str("Hello world");
        Ok(sm.get())
    }

See xxx for examples or yyy for the full API documentation.

# The CLI command

## Building the CLI

    cargo build --release

The resulting binary is named `figlet`, matching the reference implementation.

## Usage

    figlet [options] [text...]

If no text is provided, input is read from stdin.

### CLI Options

| Flag                          | Description                                    |
| ---                           | ---                                            |
| `-f, --font <name>`           | Font to use (default: `standard.flf`)          |
| `-d, --dir <dir>`             | Font directory (default: `/usr/share/figlet`)  |
| `-w, --width <cols>`          | Output width for wrapping (default: 80)        |
| `-c, --center`                | Center output                                  |
| `-l, --left`                  | Left-align output                              |
| `-r, --right`                 | Right-align output                             |
| `-o, --overlap`               | Overlap mode                                   |
| `-k, --kern`                  | Kerning mode                                   |
| `-W, --full-width`            | Full width mode (no smushing)                  |
| `-s, --smush-default`         | Smush respecting font defaults                 |
| `-S, --smush`                 | Force smush mode                               |
| `-p, --paragraph`             | Paragraph mode (ignore mid-paragraph newlines) |
| `-n, --normal`                | Normal mode (newlines are line breaks)         |
| `-R, --right-to-left`         | Right-to-left print direction                  |
| `-I, --infocode <num>`        | Print info for infocode (0-5) and exit         |
| `-v, --version`               | Print version and exit                         |
| `-h, --help`                  | Print usage and exit                           |
| `-W, --full-width`            | Display characters in full width               |
| `-w, --width <cols>`          | Set the output width                           |
| `-X, --font-direction`        | Use font file's default print direction        |
| `-x, --default-justification` | Use default justification (left for LTR, right for RTL) |

### Examples

    figlet Hello

    figlet -f small "Hello, World!"

    echo "figdriver-rs" | figlet -c -f banner

    figlet -R -f standard مرحبا

## Status

This is a work in progress. The following features from the reference
implementation are not yet supported:

- Compressed fonts (`.flf.zip`)
- CLI flags: `-x`, `-t`, `-C`, `-N`, `-I`, `-L`, `-X`
- End space in paragraph mode

The deprecated CLI flags (`-D`, `-E`) from the reference implementation will
not be implemented.
