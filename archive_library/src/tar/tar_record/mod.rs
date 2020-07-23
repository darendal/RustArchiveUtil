use crate::tar::{
    BLOCK_SIZE, DEV_MAJOR_VERSION, DEV_MINOR_VERSION, NAME_SIZE, PREFIX_SIZE, TAR_MAGIC,
    TAR_VERSION,
};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::os::macos::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use users::{get_group_by_gid, get_user_by_uid};

#[derive(Debug)]
pub struct TarRecord {
    name: String,
    mode: u32,
    user_id: u64,
    group_id: u64,
    size: u64,          // size of the file in bytes
    modified_time: i64, // Unix time file modified
    type_flag: TypeFlag,
    linkname: String,
    username: String,
    group_name: String,
    file: File,
}

impl TarRecord {
    pub fn new(path: PathBuf, root: &Path) -> TarRecord {
        let mut name = path
            .strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        if path.is_dir() {
            name.push('/');
        }

        let file = File::open(path.clone()).unwrap();
        let metadata = file.metadata().unwrap();

        let user_id = metadata.st_uid();
        let group_id = metadata.st_gid();
        let modified_time = metadata.st_mtime();

        let type_flag;
        let size;
        if path.is_dir() {
            size = 0;
            type_flag = TypeFlag::Directory;
        } else {
            size = metadata.len();
            type_flag = TypeFlag::ARegFile;
        }

        let username = get_user_by_uid(user_id).unwrap();
        let group_name = get_group_by_gid(group_id).unwrap();

        TarRecord {
            name,
            mode: (metadata.permissions().mode() & 0o07777),
            user_id: user_id as u64,
            group_id: group_id as u64,
            size,
            modified_time,
            type_flag,
            linkname: "".to_string(),
            username: username.name().to_str().unwrap().to_string(),
            group_name: group_name.name().to_str().unwrap().to_string(),
            file,
        }
    }

    pub fn write_record(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        self.write_header(writer)?;

        if self.type_flag != TypeFlag::Directory {
            self.write_file(writer)?
        }

        println!("a {}", self.name);

        Ok(())
    }

    fn write_file(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        let mut reader = BufReader::new(&self.file);
        loop {
            let buf = reader.fill_buf()?;
            let len = buf.len();
            if buf.is_empty() {
                break;
            }
            writer.write_all(buf)?;

            reader.consume(len)
        }
        let residual = BLOCK_SIZE - (self.size as usize % BLOCK_SIZE);
        if residual != BLOCK_SIZE {
            write!(writer, "{:\0<size$}", "", size = residual)?;
        }

        Ok(())
    }

    fn write_header(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        let mut vec_writer: Vec<u8> = Vec::new();

        // Write all elements of the header to the vector
        write!(
            vec_writer,
            "{name:\0<name_size$}{mode:06o} \0{user_id:06o} \0{group_id:06o} \0{size:011o} {modified_time:011o} {checksum}{typeflag}{linkname:\0<100}{magic:\0<6}{version:02}{username:\0<32}{group_name:\0<32}{dev_major:06o} \0{dev_minor:06o} \0{prefix:\0<prefix_size$}",
            name = self.name,
            name_size = NAME_SIZE,
            mode = self.mode,
            user_id = self.user_id,
            group_id = self.group_id,
            size = self.size,
            modified_time = self.modified_time,
            checksum = "        ",
            typeflag = self.type_flag as u8,
            linkname = self.linkname,
            magic = TAR_MAGIC,
            version = TAR_VERSION,
            username = self.username,
            group_name = self.group_name,
            dev_major = DEV_MAJOR_VERSION,
            dev_minor = DEV_MINOR_VERSION,
            prefix = "",
            prefix_size = PREFIX_SIZE,
        )?;

        let sum: u64 = vec_writer.iter().map(|&x| x as u64).sum();

        let mut checksum: Vec<u8> = Vec::new();
        write!(checksum, "{:06o}\0 ", sum)?;

        vec_writer[148..156].swap_with_slice(&mut checksum[0..]);
        writer.write_all(&vec_writer)?;

        // Header is exactly 12 bytes shy of a single block.
        // Write 12 nulls to fill the block before moving on.
        write!(writer, "{:\0<size$}", "", size = 12)
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[allow(dead_code)]
enum TypeFlag {
    ARegFile = b'\0',
    Link = 1,
    Directory = 5,
}
