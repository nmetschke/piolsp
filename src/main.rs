use std::{
    error::Error,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

fn format_file(file: Option<&Path>) -> io::Result<()> {
    let mut code = String::new();
    if let Some(file) = file {
        File::open(file)?.read_to_string(&mut code)?;
    } else {
        io::stdin().read_to_string(&mut code)?;
    }

    let tree = piolsp::pio_parser().parse(&code, None).unwrap();
    let formatted = piolsp::formatter::format_tree(&code, tree.root_node());
    print!("{formatted}");
    Ok(())
}

fn print_help_and_exit(msg: Option<String>) -> ! {
    if let Some(msg) = &msg {
        eprintln!("{msg}");
    }
    eprintln!(
        r#"
usage: {} (--help) (--version) (--logfile <path>) (--verbose|-v) (--pioasm <path>) (format|fmt <input_file>)
    --help                    Print help message and exit.
    --version                 Print version and exit.
    --pioasm <path>           Path to pioasm binary. Searches in $PATH if nor is specified.
    --logfile <path>          Write logs to this file.
    --verbose|-v              More verbose logging.
    fmt|format <input_file>   If fmt or format is specified, format the file and write to stdout. (WIP)
"#,
        env!("CARGO_BIN_NAME")
    );
    std::process::exit(msg.is_some() as _)
}

enum RunMode {
    Format { path: Option<PathBuf> },
    Lsp { pioasm: Option<PathBuf> },
}
struct Args {
    log_file: Option<PathBuf>,
    log_level: log::LevelFilter,
    run_mode: RunMode,
}
impl Default for Args {
    fn default() -> Self {
        Self {
            log_file: None,
            log_level: log::LevelFilter::Info,
            run_mode: RunMode::Lsp { pioasm: None },
        }
    }
}
fn parse_args() -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut next_path = None;
    let is_next_path = |next_path: &mut Option<_>, q| next_path.take_if(|e| *e == q).is_some();
    let mut next_is_fmt_file = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--help" => print_help_and_exit(None),
            "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0)
            }
            "--verbose" | "-v" => parsed.log_level = log::LevelFilter::Debug,

            path if is_next_path(&mut next_path, "pioasm") => match &mut parsed.run_mode {
                RunMode::Format { .. } => return Err("--pioasm can't be used with format".into()),
                RunMode::Lsp { pioasm } => *pioasm = Some(PathBuf::from(path)),
            },
            path if is_next_path(&mut next_path, "logfile") => {
                parsed.log_file = Some(PathBuf::from(path))
            }
            "--pioasm" => next_path = Some("pioasm"),
            "--logfile" => next_path = Some("logfile"),

            p if next_is_fmt_file && let RunMode::Format { path } = &mut parsed.run_mode => {
                *path = (!matches!(p, "" | "-")).then(|| PathBuf::from(p));
                next_is_fmt_file = false;
            }
            "fmt" | "format" if !matches!(parsed.run_mode, RunMode::Format { .. }) => {
                parsed.run_mode = RunMode::Format { path: None };
                next_is_fmt_file = true;
            }
            e => return Err(format!("unexpected arg: {e}")),
        }
    }

    if let Some(src) = next_path {
        return Err(format!("no input file for {src} provided"));
    }

    Ok(parsed)
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let run_mode = match parse_args() {
        Ok(args) => {
            if let Some(log_file) = args.log_file {
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_file)?;
                simple_logging::log_to(f, args.log_level);
            } else {
                simple_logging::log_to_stderr(args.log_level);
            }
            args.run_mode
        }
        Err(msg) => print_help_and_exit(Some(msg)),
    };

    match run_mode {
        RunMode::Format { path } => format_file(path.as_deref())?,
        RunMode::Lsp { pioasm } => piolsp::lsp::lsp(pioasm)?,
    }

    Ok(())
}
