use crate::cli::ui::Text;

pub struct Prompt {
    config: Config,
}

pub struct Config {
    compute: fn() -> Text,
}
