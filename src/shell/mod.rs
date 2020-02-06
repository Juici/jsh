use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Result;

use crate::cli::Args;

pub struct Shell {}

impl Shell {
    pub fn new(_args: Args) -> Result<Shell> {
        Ok(Shell {})
    }

    pub fn exec_command(self, _cmd: &str) -> Result<()> {
        todo!("read statements from cmd string")
    }

    pub fn exec_files(self, files: &[PathBuf]) -> Result<()> {
        for file in files {
            let file = File::open(file)?;
            let _buffer = BufReader::new(file);

            todo!("read statements from buffer");
        }

        Ok(())
    }

    pub fn interactive(self) -> Result<()> {
        // TODO: Initialize editor.

        Ok(())
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        // TODO: Restore terminal settings.
    }
}
