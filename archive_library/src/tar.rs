use crate::tar::tar_record::TarRecord;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

const TAR_MAGIC: &str = "ustar\0";
const TAR_VERSION: u32 = 0u32;
const DEV_MAJOR_VERSION: u64 = 0o0;
const DEV_MINOR_VERSION: u64 = 0o0;

const BLOCK_SIZE: usize = 512;

const NAME_SIZE: usize = 100;
const PREFIX_SIZE: usize = 155;

mod tar_record;

pub struct Tar {
    files: Vec<TarRecord>,
}

impl Tar {
    pub fn new(path: PathBuf) -> Tar {
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

            return Tar { files };
        }

        let record = TarRecord::new(path, root);

        Tar {
            files: vec![record],
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
}
