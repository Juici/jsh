use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::cli::app::{App, AppSpec, AppState, Return};
use crate::cli::prompt::{Prompt, PromptConfig, PromptModule};
use crate::cli::term::style::Color;
use crate::cli::tty::Tty;
use crate::cli::ui::Text;

pub struct Editor {
    app: App,
}

impl Editor {
    pub fn new(tty: Tty) -> Editor {
        // TODO: Namespace etc.

        let (mut prompt, prompt_handle) = Prompt::new(PromptConfig {
            threshold: Duration::from_millis(200),
        });

        prompt.add_module(Box::new(WorkingDir { wd: None }));
        prompt.add_module(Box::new(PromptMarker));

        let app_spec = AppSpec {
            tty,

            state: AppState::default(),

            // TODO: Prompts.
            prompt: Some((prompt, prompt_handle)),
            rprompt: None,
        };

        let app = App::new(app_spec);

        Editor { app }
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        self.app.read_line().await
    }
}

struct WorkingDir {
    wd: Option<PathBuf>,
}

#[async_trait]
impl PromptModule for WorkingDir {
    async fn compute(&mut self) -> Option<Text> {
        self.wd = std::env::current_dir().ok();

        match &self.wd {
            Some(dir) => dir
                .file_name()
                .map(|s| s.to_string_lossy())
                .map(|s| Text::styled(s, |style| style.fg(Color::BrightBlue).bold(true))),
            None => None,
        }
    }

    async fn should_update(&self, wd_changed: bool) -> bool {
        wd_changed
    }

    async fn update_threshold(&self) -> Option<Duration> {
        None
    }

    fn position(&self) -> isize {
        0
    }
}

struct PromptMarker;

#[async_trait]
impl PromptModule for PromptMarker {
    async fn compute(&mut self) -> Option<Text> {
        Some(Text::styled("\u{276f} ", |style| {
            style.fg(Color::BrightRed).bold(true)
        }))
    }

    async fn should_update(&self, _wd_changed: bool) -> bool {
        false
    }

    async fn update_threshold(&self) -> Option<Duration> {
        None
    }

    fn position(&self) -> isize {
        isize::max_value()
    }
}
