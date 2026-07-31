use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "haiti-lite-rust",
    disable_help_flag = true,
    disable_version_flag = true,
    arg_required_else_help = false,
    color = clap::ColorChoice::Never
)]
pub struct Cli {
    #[arg(long = "data-dir", value_name = "PATH", global = true)]
    pub data_dir: Option<PathBuf>,

    #[arg(short = 'e', long = "extended", global = true)]
    pub extended: bool,

    #[arg(long = "debug", global = true)]
    pub debug: bool,

    #[arg(long = "version")]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "hc")]
    Hc { hash: String },
    #[command(name = "jtr")]
    Jtr { hash: String },
}

#[derive(Clone, Copy, Debug)]
pub enum OutputKind {
    Hashcat,
    John,
}

impl fmt::Display for OutputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl OutputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hashcat => "hc",
            Self::John => "jtr",
        }
    }
}
