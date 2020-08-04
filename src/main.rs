#[macro_use]
extern crate clap;
extern crate archive_library;

use archive_library::tar::TarMode;
use clap::{App, ArgMatches};
use std::path::PathBuf;

fn main() -> archive_library::error::Result<()> {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand() {
        ("tar", Some(opts)) => subcommand_tar(opts),
        _ => Ok(()),
    }
}

fn subcommand_tar(args: &ArgMatches) -> archive_library::error::Result<()> {
    let filepath = PathBuf::from(args.value_of_os("input").unwrap());

    let mut destination = match args.value_of_os("output") {
        Some(o) => PathBuf::from(o),
        None => filepath.clone(),
    };

    if destination.is_dir() {
        destination.push(filepath.file_stem().unwrap())
    }

    let mut mode: TarMode = TarMode::Create;
    if args.is_present("create") {
        mode = TarMode::Create;
    } else if args.is_present("extract") {
        mode = TarMode::Extract
    } else if args.is_present("append") {
        mode = TarMode::Append
    }

    match mode {
        TarMode::Create | TarMode::Append => {
            let tar = archive_library::tar::Tar::new(filepath, mode);
            tar.write_tar(&destination)
        }
        TarMode::Extract => archive_library::tar::Tar::extract(filepath, &destination),
    }
}
