use std::process::ExitCode;

use clap::Parser;

use flippy::{app, cli::Cli};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("failed to determine current directory: {error}");
            return ExitCode::FAILURE;
        }
    };

    match app::run(cli, &cwd, &mut std::io::stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
