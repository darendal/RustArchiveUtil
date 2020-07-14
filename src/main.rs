use std::{env, process, path};
use std::ffi::OsString;

const UTIL_NAME: &str = "Rust Archive Util";

fn main() {
    let args: Vec<OsString> = env::args_os().collect();

    if args.len() < 2 {
        eprintln!("{} - Missing Required argument <File path>", UTIL_NAME);
        process::exit(1);
    }

    let filepath = path::Path::new(&args[1]);

    if !filepath.exists() {
        eprintln!("{} - Parameter {:?}: File does not exist", UTIL_NAME, filepath);
        process::exit(1);
    }

    println!("{:?}", filepath);
}
