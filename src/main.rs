#![allow(dead_code)]

mod args;
mod cli;
mod editor;
mod shell;

use anyhow::Result;

use crate::args::LaunchMode;
use crate::shell::Shell;

#[tokio::main]
async fn main() -> Result<()> {
    let (mode, args) = args::args();
    let shell = Shell::new(args)?;

    // TODO: Add logging to file.

    match mode {
        LaunchMode::Exec(cmd) => shell.exec_command(&cmd).await,
        LaunchMode::Files(files) => shell.exec_files(&files).await,
        LaunchMode::Interactive => shell.interactive().await,
    }
}
