use std::{io::Write, path::Path, process::Stdio};

use tree_sitter::{Language, Parser};

pub mod doc;
pub mod formatter;
pub mod lsp;

pub fn pio_lang() -> Language {
    tree_sitter_pio::LANGUAGE.into()
}

pub fn pio_parser() -> Parser {
    let mut parser = tree_sitter::Parser::new();
    if let Err(err) = parser.set_language(&pio_lang()) {
        let msg = format!(
            "failed to load language, language version is {}, compatible range is {}..={}: {err}",
            pio_lang().abi_version(),
            tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION,
            tree_sitter::LANGUAGE_VERSION
        );
        log::error!("{msg}");
        panic!("{msg}");
    }
    parser
}

pub fn pio_query(source: &str) -> tree_sitter::Query {
    tree_sitter::Query::new(&pio_lang(), source).unwrap()
}

pub fn run_pioasm(src: &str, pioasm: Option<&Path>) -> std::io::Result<std::process::Output> {
    let mut pioasm = std::process::Command::new(pioasm.map_or("pioasm".as_ref(), Path::as_os_str));
    pioasm.args(["-o", "json", ""]); // "" to read from stdin
    pioasm
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    log::debug!(
        "running '{} {}'",
        pioasm.get_program().to_str().unwrap(),
        pioasm.get_args().fold(String::new(), |mut acc, e| {
            if !acc.is_empty() {
                acc.push(' ');
            }
            acc += e.to_str().unwrap();
            acc
        })
    );

    let mut child = pioasm.spawn()?;
    child.stdin.as_mut().unwrap().write_all(src.as_bytes())?;
    child.wait_with_output()
}
