use crate::tar::tar_record::TarRecord;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::{fs, io};
use walkdir::WalkDir;

const TAR_MAGIC: &str = "ustar\0";
const TAR_VERSION: u32 = 0u32;
const DEV_MAJOR_VERSION: u64 = 0o0;
const DEV_MINOR_VERSION: u64 = 0o0;

const BLOCK_SIZE: usize = 512;

mod tar_record;

pub struct Tar {
    files: Vec<TarRecord>,
}

impl Tar {
    pub fn new(path: PathBuf, mode: TarMode) -> Tar {
        match mode {
            TarMode::Create => Tar::create(path),
            m => panic!("Unsupported mode: {:?}", m),
        }
    }

    pub fn write_tar(&self, path: &PathBuf) -> Result<(), io::Error> {
        let mut result_path = path.clone();
        result_path.set_extension("tar");
        let mut writer = BufWriter::new(File::create(result_path)?);

        for record in self.files.iter() {
            record.write_record(&mut writer)?
        }

        // write 2 empty blocks to signify end of TAR
        write!(writer, "{:\0<size$}", "", size = BLOCK_SIZE * 2)?;

        writer.flush()
    }

    fn create(path: PathBuf) -> Tar {
        let mut root = path.clone();
        root.pop();
        let root = root.as_path();

        if path.is_dir() {
            let files: Vec<TarRecord> = WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !crate::is_hidden(e))
                .filter_map(|e| e.ok())
                .map(|file| TarRecord::new(file.into_path(), root))
                .collect();

            Tar { files }
        } else {
            let record = TarRecord::new(path, root);

            Tar {
                files: vec![record],
            }
        }
    }

    pub fn extract(input: PathBuf, output: &PathBuf) -> Result<(), io::Error> {
        if input.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected tar file, found directory",
            ));
        };

        match input.extension() {
            Some(ext) => {
                if ext.ne("tar") {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Expected tar file, found {}", ext.to_string_lossy()),
                    ));
                }
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "input file missing extension. Unknown if is a tar file",
                ))
            }
        };

        let file = File::open(input)?;
        let mut reader = BufReader::new(file);

        fs::create_dir_all(&output)?;

        TarRecord::new_from_file(&mut reader, output)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum TarMode {
    Create,
    Extract,
    Append,
}
