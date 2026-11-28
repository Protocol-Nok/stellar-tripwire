//! `tripwire`: generate, verify and run the ledger conformance corpus.

mod replay_run;
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
    ///
    /// The adapter command goes after `--`, so it can contain its own flags (`java -jar
    /// x.jar`, `node --experimental-foo x.mjs`) without colliding with tripwire's own.
    /// The corpus directory is appended as its last argument.
    ///
    ///   tripwire run <dir> [--json] -- <adapter-cmd...>
    Run {
        dir: PathBuf,
        /// Print the report as JSON instead of text.
        #[arg(long)]
        json: bool,
        #[arg(last = true, required = true)]
        adapter: Vec<String>,
    },
    /// The indexer replay harness: checks whether an *indexer* extracts the right facts
    /// from a whole ledger, not whether an SDK decodes one value (that's `generate`/
    /// `verify`/`run`, above). See docs/replay-protocol.md.
    #[command(subcommand)]
    Replay(ReplayCommand),
}

#[derive(Subcommand)]
enum ReplayCommand {
    /// Write the replay corpus (manifest.json and entries/*.xdr, one LedgerCloseMeta per
    /// entry) into DIR.
    Generate { dir: PathBuf },
    /// Check that a replay corpus decodes as LedgerCloseMeta, round-trips, and matches
    /// its manifest.
    Verify {
        dir: PathBuf,
        #[arg(long)]
        regen: bool,
    },
    /// Run an indexer adapter against the replay corpus.
    ///
    ///   tripwire replay run <dir> [--json] -- <indexer-cmd...>
    Run {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(last = true, required = true)]
        adapter: Vec<String>,
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
        Command::Replay(ReplayCommand::Generate { dir }) => {
            let m = tripwire_replay::write_replay_corpus(&dir)?;
            println!(
                "wrote {} entries to {} ({})",
                m.entries.len(),
                dir.display(),
                m.xdr_crate
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Replay(ReplayCommand::Verify { dir, regen }) => {
            let findings = tripwire_replay::verify_dir(&dir, regen)?;
            if findings.is_empty() {
                println!("ok: replay corpus is consistent");
                return Ok(ExitCode::SUCCESS);
            }
            for f in &findings {
                println!("{}: {}", f.entry, f.message);
            }
            Ok(ExitCode::from(1))
        }
        Command::Replay(ReplayCommand::Run { dir, adapter, json }) => {
            replay_run::run(&dir, &adapter, json)
        }
    }
}
