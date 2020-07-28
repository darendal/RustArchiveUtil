use crate::tar::{BLOCK_SIZE, DEV_MAJOR_VERSION, DEV_MINOR_VERSION, TAR_MAGIC, TAR_VERSION};
use filetime::FileTime;
use std::fs::{File};
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::macos::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use users::{get_group_by_gid, get_user_by_uid};

const NAME_OFFSET: usize = 0;
const NAME_SIZE: usize = 100;

const MODE_OFFSET: usize = 100;
const MODE_SIZE: usize = 8;

const UID_OFFSET: usize = 108;
const UID_SIZE: usize = 8;

const GID_OFFSET: usize = 116;
const GID_SIZE: usize = 8;

const SIZE_OFFSET: usize = 124;
const SIZE_SIZE: usize = 12;

const MODIFIED_TIME_OFFSET: usize = 136;
const MODIFIED_TIME_SIZE: usize = 12;

const CHECKSUM_OFFSET: usize = 148;
const CHECKSUM_SIZE: usize = 8;
const CHECKSUM_DEFAULT: &str = "        ";

const TYPEFLAG_OFFSET: usize = 156;

const LINKNAME_OFFSET: usize = 157;
const LINKNAME_SIZE: usize = 100;

const MAGIC_OFFSET: usize = 257;
const MAGIC_SIZE: usize = 6;

const UNAME_OFFSET: usize = 265;
const UNAME_SIZE: usize = 32;

const GNAME_OFFSET: usize = 297;
const GNAME_SIZE: usize = 32;

const PREFIX_SIZE: usize = 155;

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
    file: Option<File>,
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
        let modified_time = FileTime::from_last_modification_time(&metadata).unix_seconds();

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
            file: Some(file),
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
        let file = self.file.as_ref().unwrap();
        let mut reader = BufReader::new(file);
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
        let mut vec_writer: Vec<u8> = Vec::with_capacity(crate::tar::BLOCK_SIZE);

        // Write all elements of the header to the vector
        write!(
            vec_writer,
            "{name:\0<name_size$}{mode:06o} \0{user_id:06o} \0{group_id:06o} \0{size:011o} {modified_time:011o} {checksum}{typeflag}{linkname:\0<linkname_size$}{magic:\0<6}{version:02}{username:\0<32}{group_name:\0<32}{dev_major:06o} \0{dev_minor:06o} \0{prefix:\0<prefix_size$}",
            name = self.name,
            name_size = NAME_SIZE,
            mode = self.mode,
            user_id = self.user_id,
            group_id = self.group_id,
            size = self.size,
            modified_time = self.modified_time,
            checksum = CHECKSUM_DEFAULT,
            typeflag = self.type_flag as u8,
            linkname = self.linkname,
            linkname_size = LINKNAME_SIZE,
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

    pub fn new_from_file(reader: &mut impl Read, output: &PathBuf) -> Result<(), io::Error> {
        let record = TarRecord::read_header(reader)?;
        record.extract_file(reader, &mut output.clone())
    }

    fn read_header(reader: &mut impl Read) -> Result<TarRecord, io::Error> {
        let mut header_block = [0; crate::tar::BLOCK_SIZE];

        reader.read_exact(&mut header_block).unwrap();

        let magic =
            String::from_utf8_lossy(&header_block[MAGIC_OFFSET..(MAGIC_OFFSET + MAGIC_SIZE)]);

        if magic.ne(crate::tar::TAR_MAGIC) {
            // Something is wrong with the header
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "header corrupted, invalid magic value.",
            ));
        }

        let checksum = String::from_utf8_lossy(
            &header_block[CHECKSUM_OFFSET..(CHECKSUM_OFFSET + CHECKSUM_SIZE - 2)],
        );
        let checksum = u64::from_str_radix(&checksum, 8).unwrap();

        // Replace checksum with default before checking if header is valid
        header_block[CHECKSUM_OFFSET..(CHECKSUM_SIZE + CHECKSUM_OFFSET)]
            .copy_from_slice(CHECKSUM_DEFAULT.as_bytes());

        let sum: u64 = header_block.iter().map(|&x| x as u64).sum();

        if checksum != sum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Checksum is invalid. File may be corrupted.",
            ));
        }

        let name = String::from_utf8_lossy(&header_block[NAME_OFFSET..(NAME_SIZE + NAME_OFFSET)]);
        let mode = u32::from_str_radix(
            &String::from_utf8_lossy(&header_block[MODE_OFFSET..(MODE_OFFSET + MODE_SIZE) - 2]),
            8,
        )
        .unwrap();
        let user_id = u64::from_str_radix(
            &String::from_utf8_lossy(&header_block[UID_OFFSET..(UID_OFFSET + UID_SIZE) - 2]),
            8,
        )
        .unwrap();
        let group_id = u64::from_str_radix(
            &String::from_utf8_lossy(&header_block[GID_OFFSET..(GID_OFFSET + GID_SIZE) - 2]),
            8,
        )
        .unwrap();
        let size = u64::from_str_radix(
            &String::from_utf8_lossy(&header_block[SIZE_OFFSET..(SIZE_OFFSET + SIZE_SIZE) - 1]),
            8,
        )
        .unwrap();
        let modified_time = i64::from_str_radix(
            &String::from_utf8_lossy(
                &header_block
                    [MODIFIED_TIME_OFFSET..(MODIFIED_TIME_OFFSET + MODIFIED_TIME_SIZE) - 1],
            ),
            8,
        )
        .unwrap();

        let username =
            String::from_utf8_lossy(&header_block[UNAME_OFFSET..(UNAME_OFFSET + UNAME_SIZE)]);
        let group_name =
            String::from_utf8_lossy(&header_block[GNAME_OFFSET..(GNAME_OFFSET + GNAME_SIZE)]);

        let type_flag_u8 = u8::from_str(&String::from_utf8_lossy(
            &header_block[TYPEFLAG_OFFSET..=TYPEFLAG_OFFSET],
        ))
        .unwrap();

        let type_flag = if type_flag_u8 == TypeFlag::ARegFile as u8 {
            TypeFlag::ARegFile
        } else if type_flag_u8 == TypeFlag::Directory as u8 {
            TypeFlag::Directory
        } else {
            TypeFlag::Link
        };

        Ok(TarRecord {
            name: name.trim_end_matches('\0').to_string(),
            mode,
            user_id,
            group_id,
            size,
            modified_time,
            type_flag,
            linkname: "".to_string(),
            username: username.trim_end_matches('\0').to_string(),
            group_name: group_name.trim_end_matches('\0').to_string(),
            file: None,
        })
    }

    fn extract_file(&self, reader: &mut impl Read, output: &mut PathBuf) -> Result<(), io::Error> {
        output.push(Path::new(&self.name));

        if self.type_flag == TypeFlag::Directory {
            std::fs::create_dir_all(&output).unwrap();
        }

        let file: File = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(output.clone())?;

        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(self.mode);
        file.set_permissions(permissions)?;

        let mut writer = BufWriter::new(file);

        let mut remaining = self.size as usize;
        let mut block = [0; crate::tar::BLOCK_SIZE];

        while remaining > 0 {
            reader.read_exact(&mut block)?;

            if remaining < crate::tar::BLOCK_SIZE {
                writer.write_all(&block[0..remaining])?;
                remaining = 0;
            } else {
                remaining -= crate::tar::BLOCK_SIZE;
                writer.write_all(&block)?;
            }
        }

        writer.flush()?;

        filetime::set_file_mtime(&output, FileTime::from_unix_time(self.modified_time, 0))?;
        Ok(())
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
