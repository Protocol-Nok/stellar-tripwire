//! `tripwire`: generate, verify and run the ledger conformance corpus.

mod run;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tripwire",
    version,
    about = "Ledger conformance corpus for Stellar SDKs and indexers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write the corpus (manifest.json and entries/*.xdr) into DIR.
    Generate { dir: PathBuf },
    /// Check that a corpus decodes, round-trips and matches its manifest.
    Verify {
        dir: PathBuf,
        /// Also require the files to equal what the generator produces today.
        #[arg(long)]
        regen: bool,
    },
    /// Run an adapter (any program that speaks the adapter protocol) against the corpus.
    Run {
        dir: PathBuf,
        /// Adapter command and its arguments. The corpus directory is appended last.
        #[arg(long, required = true, num_args = 1.., allow_hyphen_values = true)]
        adapter: Vec<String>,
        /// Print the report as JSON instead of text.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Generate { dir } => {
            let m = tripwire_corpus::write_corpus(&dir)?;
            println!(
                "wrote {} entries to {} ({})",
                m.entries.len(),
                dir.display(),
                m.xdr_crate
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Verify { dir, regen } => {
            let findings = tripwire_corpus::verify_dir(&dir, regen)?;
            if findings.is_empty() {
                println!("ok: corpus is consistent");
                return Ok(ExitCode::SUCCESS);
            }
            for f in &findings {
                println!("{}: {}", f.entry, f.message);
            }
            Ok(ExitCode::from(1))
        }
        Command::Run { dir, adapter, json } => run::run(&dir, &adapter, json),
    }
}
