extern crate archive_library;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, process};

const UTIL_NAME: &str = "Rust Archive Util";

fn main() {
    let filepath = match fetch_path(env::args_os().collect()) {
        None => {
            eprintln!("{} - Missing Required argument <File path>", UTIL_NAME);
            process::exit(1);
        }
        Some(path) => path,
    };
    let mut destination = filepath.clone();
    if !valid_path(filepath.as_path()) {
        eprintln!(
            "{} - Parameter {:?}: File does not exist",
            UTIL_NAME, filepath
        );
        process::exit(1);
    }

    let tar = archive_library::tar::Tar::new(filepath);

    match tar.write_tar(&mut destination) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error writing tar: {}", e);
            process::exit(1);
        },
    };

}

fn fetch_path(args: Vec<OsString>) -> Option<PathBuf> {
    if args.len() < 2 {
        None
    } else {
        Some(PathBuf::from(&args[1]))
    }
}

fn valid_path(filepath: &Path) -> bool {
    filepath.exists()
}

#[test]
fn fetch_path_test() {
    let expected_none = fetch_path(Vec::new());
    assert_eq!(expected_none, None);

    let args: Vec<OsString> = vec![OsString::from("test"), OsString::from("test2")];

    match fetch_path(args) {
        None => assert!(false),
        Some(p) => assert_eq!(p.as_path().as_os_str(), OsString::from("test2")),
    }
}

#[test]
fn test_valid_path() {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/resources");

    let mut valid = PathBuf::from(d.clone());
    valid.push("test.txt");

    let mut invalid = PathBuf::from(d);
    invalid.push("invalid.txt");

    assert_eq!(valid_path(valid.as_path()), true);
    assert_eq!(valid_path(invalid.as_path()), false);
}
