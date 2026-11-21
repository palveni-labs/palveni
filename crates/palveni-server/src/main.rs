//! `palveni`: the Palveni Server executable.

mod cli;
mod doctor;
mod library_monitor;
mod logging;
mod scan_job;
mod serve;
#[cfg(feature = "embed-web")]
mod web;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve::run(&cli).await,
        Command::Doctor => doctor::run(&cli).await,
        Command::Openapi => {
            serde_json::to_writer_pretty(std::io::stdout().lock(), palveni_api::openapi_document())
                .map_err(Into::into)
        }
        Command::Version => {
            print_version();
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn print_version() {
    println!(
        "palveni {} ({}/{})",
        palveni_core::VERSION,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}
