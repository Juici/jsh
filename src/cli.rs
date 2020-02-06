use std::path::PathBuf;

use clap::{App, AppSettings, Arg};

mod arg {
    pub const VERBOSE: &str = "verbose";

    pub const EXEC: &str = "exec";
    pub const FILES: &str = "files";
}

fn app() -> App<'static, 'static> {
    App::new(pkg::name!())
        .version(pkg::version!())
        .author(pkg::authors!("\n"))
        .about(concat!("\n", pkg::description!()))
        .setting(AppSettings::ColorAuto)
        .arg(
            Arg::with_name(arg::VERBOSE)
                .help("Enables verbose output")
                .short("v")
                .long("verbose"),
        )
        .arg(
            Arg::with_name(arg::EXEC)
                .help("Takes the first argument as a command to execute")
                .short("c")
                .long("exec")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(arg::FILES)
                .conflicts_with(arg::EXEC)
                .help("Script files to execute")
                .value_name("FILE")
                .min_values(0)
                .required(false),
        )
}

pub enum LaunchMode {
    Exec(String),
    Files(Vec<PathBuf>),
    Interactive,
}

pub struct Args {
    pub verbose: bool,
}

pub fn args() -> (LaunchMode, Args) {
    let matches = app().get_matches();

    let verbose = matches.is_present(arg::VERBOSE);

    let args = Args { verbose };

    let mode = match matches.value_of(arg::EXEC) {
        Some(cmd) => LaunchMode::Exec(cmd.to_owned()),
        None => match matches.values_of_os(arg::FILES) {
            Some(files) => LaunchMode::Files(files.map(PathBuf::from).collect()),
            None => LaunchMode::Interactive,
        },
    };

    (mode, args)
}
