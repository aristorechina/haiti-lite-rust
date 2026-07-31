use std::io::{self, Read};
use std::process::ExitCode;

use clap::Parser;
use serde::Serialize;
use thiserror::Error;

use haiti_lite_rust::cli::{Cli, Command, OutputKind};
use haiti_lite_rust::data::{DataError, DataSet};
use haiti_lite_rust::matcher::{CompiledRules, MatchError, MatchRecord};
use haiti_lite_rust::ruby_chomp;

#[derive(Debug, Error)]
enum AppError {
    #[error("{0}")]
    Cli(#[from] clap::Error),
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    Data(#[from] DataError),
    #[error("{0}")]
    Match(#[from] MatchError),
    #[error("could not read hash from standard input: {0}")]
    Stdin(#[source] io::Error),
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct DebugResponse {
    data_dir: String,
    extended: bool,
}

#[derive(Debug, Serialize)]
struct MatchResponse {
    mode: &'static str,
    hash: String,
    identified: bool,
    matches: Vec<MatchRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<DebugResponse>,
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", serde_json::json!({"error": error.to_string()}));
            ExitCode::from(2)
        }
    }
}

fn execute() -> Result<(), AppError> {
    let cli = Cli::try_parse()?;

    if cli.version {
        let response = VersionResponse {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
        };
        println!(
            "{}",
            serde_json::to_string(&response).expect("version is serializable")
        );
        return Ok(());
    }

    let command = cli
        .command
        .ok_or_else(|| AppError::Usage("a mode (`hc` or `jtr`) and hash are required".into()))?;
    let data_dir = cli
        .data_dir
        .ok_or_else(|| AppError::Usage("--data-dir <path> is required for matching".into()))?;
    let data = DataSet::load(&data_dir)?;
    let rules = CompiledRules::compile(&data)?;
    let (output_kind, raw_hash) = match command {
        Command::Hc { hash } => (OutputKind::Hashcat, hash),
        Command::Jtr { hash } => (OutputKind::John, hash),
    };
    let hash = read_hash(&raw_hash)?;

    let matches = rules.identify(&hash)?;
    let response = MatchResponse {
        mode: output_kind.as_str(),
        hash,
        identified: !matches.is_empty(),
        matches: rules.render(&matches, output_kind, cli.extended),
        debug: cli.debug.then(|| DebugResponse {
            data_dir: data_dir.display().to_string(),
            extended: cli.extended,
        }),
    };
    println!(
        "{}",
        serde_json::to_string(&response).expect("match response is serializable")
    );

    Ok(())
}

fn read_hash(raw_hash: &str) -> Result<String, AppError> {
    if raw_hash != "-" {
        return Ok(raw_hash.to_owned());
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(AppError::Stdin)?;
    Ok(ruby_chomp(&input))
}
