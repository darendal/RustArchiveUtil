#[macro_use]
extern crate clap;
extern crate archive_library;

use clap::{App, ArgMatches};
use std::path::{Path, PathBuf};
use std::io;

const UTIL_NAME: &str = "Rust Archive Util";

fn main() -> Result<(), io::Error> {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand() {
        ("tar", Some(opts)) => subcommand_tar(opts),
        _ => Ok(()),
    }
}

fn valid_path(filepath: &Path) -> bool {
    filepath.exists()
}

fn subcommand_tar(args: &ArgMatches) -> Result<(), io::Error> {
    let filepath = PathBuf::from(args.value_of_os("input").unwrap());

    if !valid_path(filepath.as_path()) {
        eprintln!(
            "{} - Parameter {:?}: File does not exist",
            UTIL_NAME, filepath
        );
    }

    let mut destination = match args.value_of_os("output") {
        Some(o) => PathBuf::from(o),
        None => filepath.clone()
    };

    if destination.is_dir() {
        destination.push(filepath.file_stem().unwrap())
    }

    let tar = archive_library::tar::Tar::new(filepath);

    tar.write_tar(&destination)
}
