use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Result;

use crate::args::Args;
use crate::cli::app::Return;
use crate::cli::tty::Tty;
use crate::editor::Editor;

pub struct Shell {}

impl Shell {
    pub fn new(_args: Args) -> Result<Shell> {
        Ok(Shell {})
    }

    pub async fn exec_command(self, _cmd: &str) -> Result<()> {
        todo!("read statements from cmd string")
    }

    pub async fn exec_files(self, files: &[PathBuf]) -> Result<()> {
        for file in files {
            let file = File::open(file)?;
            let _buffer = BufReader::new(file);

            todo!("read statements from buffer");
        }

        Ok(())
    }

    pub async fn interactive(self) -> Result<()> {
        // TODO: Check isatty.
        let mut editor = Editor::new(Tty::std());

        // TODO: Source config files.

        // TODO: Initialize editor.

        loop {
            let line = editor.read_line().await?;

            match line {
                Return::Input(line) => println!("read line: {}", line),
                Return::Break => println!("break"),
                Return::Exit => {
                    println!("exit");
                    return Ok(());
                }
            }
        }
    }
}
