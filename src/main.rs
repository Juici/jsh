mod cli;
mod shell;

use anyhow::Result;

use crate::cli::LaunchMode;
use crate::shell::Shell;

fn main() -> Result<()> {
    let (mode, args) = cli::args();
    let shell = Shell::new(args)?;

    // TODO: Add logging to file.

    match mode {
        LaunchMode::Exec(cmd) => shell.exec_command(&cmd),
        LaunchMode::Files(files) => shell.exec_files(&files),
        LaunchMode::Interactive => shell.interactive(),
    }
}
