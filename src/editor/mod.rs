use anyhow::Result;

use crate::cli::app::{App, AppSpec, Return};
use crate::cli::tty::Tty;

pub struct Editor {
    app: App,
}

impl Editor {
    pub fn new(tty: Tty) -> Editor {
        // TODO: Namespace etc.

        let app_spec = AppSpec { tty };

        let app = App::new(app_spec);

        Editor { app }
    }

    pub async fn read_line(&mut self) -> Result<Return> {
        self.app.read_line().await
    }
}
