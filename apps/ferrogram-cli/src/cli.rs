//! Commands, argument parsing and execution (ADR 0026).
//!
//! Parsing produces a [`Command`]; `run` executes it through library calls.
//! The `Command` enum is the seam to the Phase 4 command registry
//! (`crates/commands`): each variant maps to a registry command id then,
//! and the parser and exit codes do not change.
//!
//! Nothing writes to stdout except `inspect` output and `--help`
//! (ADR 0026); errors and warnings go to stderr, one line each.

use std::ffi::OsString;
use std::path::PathBuf;

mod exec;
mod inspect;
mod parse;

use ferrogram_render::Rgba;

/// The output formats the CLI writes (ADR 0026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputFormat {
    /// PNG raster (ADR 0025).
    Png,
    /// SVG document (ADR 0025).
    Svg,
}

/// The page background `render --background` asks for (ADR 0026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Background {
    /// `--background none`: a transparent page.
    Transparent,
    /// `--background #RRGGBB`: an opaque page colour.
    Color(Rgba),
}

/// Arguments of `inspect` (ADR 0026).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspect {
    /// The input file.
    pub file: PathBuf,
    /// Whether to print JSON instead of text.
    pub json: bool,
    /// The page to summarise (1-based), or all pages.
    pub page: Option<u32>,
}

/// Arguments of `render` (ADR 0026).
#[derive(Debug, Clone, PartialEq)]
pub struct Render {
    /// The input file.
    pub file: PathBuf,
    /// The output file.
    pub out: PathBuf,
    /// The page to render (1-based), or the first page.
    pub page: Option<u32>,
    /// The output format.
    pub format: OutputFormat,
    /// Pixels per inch (ADR 0025).
    pub scale: f64,
    /// The page background override (`--background`), or the page's own
    /// background.
    pub background: Option<Background>,
    /// Whether warnings fail the run (ADR 0021).
    pub strict: bool,
}

/// Arguments of `export` (ADR 0026).
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// The input file.
    pub file: PathBuf,
    /// The output directory.
    pub out: PathBuf,
    /// The output format.
    pub format: OutputFormat,
    /// Pixels per inch (ADR 0025).
    pub scale: f64,
    /// Whether warnings fail the run (ADR 0021).
    pub strict: bool,
}

/// One CLI command (ADR 0026). Each variant maps to a registry command id
/// in Phase 4 (`file.inspect`, `file.render`, `file.export`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Command {
    /// Summarise a document.
    Inspect(Inspect),
    /// Render one page.
    Render(Render),
    /// Render every page into a directory.
    Export(Export),
}

/// The process exit codes (ADR 0026).
pub mod exit {
    /// Success.
    pub const OK: i32 = 0;
    /// Usage error (bad arguments).
    pub const USAGE: i32 = 1;
    /// Input error (unreadable or unparsable file).
    pub const READ: i32 = 2;
    /// Render or output error.
    pub const RENDER: i32 = 3;
    /// Warnings occurred and `--strict` is set.
    pub const WARNINGS: i32 = 4;
}

/// A usage error: bad arguments, with a message for stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// The message, written to stderr.
    pub message: String,
}

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// A command failure (ADR 0026).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input file could not be read or parsed.
    Read(String),
    /// Rendering failed.
    Render(String),
    /// The output could not be written.
    Io(String),
}

impl Error {
    /// The exit code of this failure (ADR 0026).
    ///
    /// [`Error::Read`] is the input error (exit 2); [`Error::Render`] and
    /// [`Error::Io`] are render or output errors (exit 3).
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Read(_) => exit::READ,
            Error::Render(_) | Error::Io(_) => exit::RENDER,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Read(m) => write!(f, "cannot read input: {m}"),
            Error::Render(m) => write!(f, "render failed: {m}"),
            Error::Io(m) => write!(f, "cannot write output: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Parses argv (without `argv[0]`) into a command.
///
/// `Ok(None)` means help was requested and has been written to stdout. An
/// unknown or missing argument is a [`Usage`] whose message and usage text
/// the caller writes to stderr (exit code 1, ADR 0026).
pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Option<Command>, Usage> {
    parse::parse_to(args, &mut std::io::stdout())
}

/// Runs a command and returns the process exit code (ADR 0026).
///
/// Returns [`exit::OK`], or [`exit::WARNINGS`] when warnings occurred and
/// the command is `--strict` (the output is still written). A failure is
/// printed to stderr as `error: <message>` and mapped to its exit code by
/// [`Error::exit_code`]: [`exit::READ`] for input errors, [`exit::RENDER`]
/// for render and output errors. `Err` is returned only when that report
/// itself cannot be written.
pub fn run(command: &Command) -> Result<i32, Error> {
    exec::run_to(command, &mut std::io::stdout(), &mut std::io::stderr())
}

/// The entry point used by `main` and by tests: parse, run, map to an exit
/// code.
///
/// A [`Usage`] error prints its message and the usage text to stderr and
/// returns [`exit::USAGE`]; `--help` prints the usage text to stdout and
/// returns [`exit::OK`].
pub fn main_from(args: impl Iterator<Item = OsString>) -> Result<i32, Error> {
    match parse(args) {
        Ok(None) => Ok(exit::OK),
        Ok(Some(command)) => run(&command),
        Err(usage) => {
            let mut err = std::io::stderr();
            exec::report(&mut err, &format!("error: {usage}"))
                .and_then(|()| exec::report(&mut err, parse::USAGE_TEXT))
                .map(|()| exit::USAGE)
                .map_err(|error| Error::Io(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_and_errors_display_their_messages() {
        let u = Usage {
            message: "bad flag".to_owned(),
        };
        assert_eq!(u.to_string(), "bad flag");
        assert_eq!(
            Error::Read("x".to_owned()).to_string(),
            "cannot read input: x"
        );
        assert_eq!(
            Error::Render("x".to_owned()).to_string(),
            "render failed: x"
        );
        assert_eq!(
            Error::Io("x".to_owned()).to_string(),
            "cannot write output: x"
        );
    }

    #[test]
    fn error_is_a_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(Error::Render("x".to_owned()));
        assert_eq!(e.to_string(), "render failed: x");
    }

    #[test]
    fn error_exit_codes_follow_adr_0026() {
        assert_eq!(Error::Read(String::new()).exit_code(), exit::READ);
        assert_eq!(Error::Render(String::new()).exit_code(), exit::RENDER);
        assert_eq!(Error::Io(String::new()).exit_code(), exit::RENDER);
    }

    #[test]
    fn commands_hold_their_arguments() {
        let c = Command::Inspect(Inspect {
            file: PathBuf::from("a.vsdx"),
            json: true,
            page: Some(2),
        });
        match c {
            Command::Inspect(i) => {
                assert!(i.json);
                assert_eq!(i.page, Some(2));
            }
            _ => panic!("expected inspect"),
        }
        let r = Command::Render(Render {
            file: PathBuf::from("a.vsdx"),
            out: PathBuf::from("a.png"),
            page: None,
            format: OutputFormat::Png,
            scale: 96.0,
            background: Some(Background::Transparent),
            strict: false,
        });
        match r {
            Command::Render(r) => {
                assert_eq!(r.scale, 96.0);
                assert_eq!(r.background, Some(Background::Transparent));
            }
            _ => panic!("expected render"),
        }
        let e = Command::Export(Export {
            file: PathBuf::from("a.vsdx"),
            out: PathBuf::from("out"),
            format: OutputFormat::Svg,
            scale: 300.0,
            strict: true,
        });
        match e {
            Command::Export(e) => assert!(e.strict),
            _ => panic!("expected export"),
        }
    }

    #[test]
    fn exit_codes_are_fixed() {
        assert_eq!(exit::OK, 0);
        assert_eq!(exit::USAGE, 1);
        assert_eq!(exit::READ, 2);
        assert_eq!(exit::RENDER, 3);
        assert_eq!(exit::WARNINGS, 4);
    }

    #[test]
    fn background_holds_its_colour() {
        let b = Background::Color(Rgba::opaque(0x12, 0x34, 0x56));
        match b {
            Background::Color(c) => assert_eq!(c, Rgba::opaque(0x12, 0x34, 0x56)),
            _ => panic!("expected a colour"),
        }
    }
}
