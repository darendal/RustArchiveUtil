use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::macos::fs::MetadataExt;
use std::path::PathBuf;
use users::{get_group_by_gid, get_user_by_uid};

const TAR_MAGIC: &str = "ustar\0";
const TAR_VERSION: u32 = 0u32;
const DEV_MAJOR_VERSION: u64 = 0o0;
const DEV_MINOR_VERSION: u64 = 0o0;

const BLOCK_SIZE: usize = 512;

const NAME_SIZE: usize = 100;
const PREFIX_SIZE: usize = 155;

pub struct Tar {
    files: Vec<TarRecord>,
}

impl Tar {
    pub fn new(path: PathBuf) -> Tar {
        if path.is_dir() {
            panic!("Currently no support for directories");
        }

        let record = TarRecord::new(path.clone());

        Tar {
            files: vec![record],
        }
    }

    pub fn write_tar(&self, path: &mut PathBuf) -> Result<(), io::Error> {
        let result_path = path;
        result_path.set_extension("tar");
        let mut writer = BufWriter::new(File::create(result_path).unwrap());

        for record in self.files.iter() {
            record.write_record(&mut writer)?
        }

        // write 2 empty blocks to signify end of TAR
        write!(writer, "{:\0<size$}", "", size = BLOCK_SIZE * 2)
    }
}

#[derive(Debug)]
struct TarRecord {
    name: String,
    mode: u64,
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
    pub fn new(path: PathBuf) -> TarRecord {
        println!("making record for file {:?}", path);
        let name = path.file_name().unwrap();

        let file = File::open(path.clone()).unwrap();
        let metadata = file.metadata().unwrap();

        let user_id = metadata.st_uid();
        let group_id = metadata.st_gid();
        let size = metadata.len();
        let modified_time = metadata.st_mtime();
        let type_flag = TypeFlag::ARegFile;

        let username = get_user_by_uid(user_id).unwrap();
        let group_name = get_group_by_gid(group_id).unwrap();

        TarRecord {
            name: name.to_str().unwrap().to_string(),
            mode: Mode::READ_BY_OWNER.bits
                | Mode::WRITE_BY_OWNER.bits
                | Mode::READ_BY_GROUP.bits
                | Mode::READ_BY_OTHER.bits,
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
        self.write_file(writer)
    }

    fn write_file(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        let mut reader = BufReader::new(&self.file);
        loop {
            let buf = reader.fill_buf()?;
            let len = buf.len();
            if buf.len() == 0 {
                break;
            }
            writer.write_all(buf)?;

            reader.consume(len)
        }
        let residual = BLOCK_SIZE - (self.size as usize % BLOCK_SIZE);
        if residual != BLOCK_SIZE {
            write!(writer, "{:\0<size$}", "", size = residual)?;
        }

        return Ok(());
    }

    fn write_header(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        let mut vec_writer: Vec<u8> = Vec::new();

        // Write all elements of the header to the vector
        write!(
            vec_writer,
            "{name:\0<name_size$}{mode:06o} \0{user_id:06o} \0{group_id:06o} \0{size:011o} {modified_time:011o} {checksum:06o}\0 {typeflag}{linkname:\0<100}{magic:\0<6}{version:02}{username:\0<32}{group_name:\0<32}{dev_major:06o} \0{dev_minor:06o} \0{prefix:\0<prefix_size$}",
            name = self.name,
            name_size = NAME_SIZE,
            mode = self.mode,
            user_id = self.user_id,
            group_id = self.group_id,
            size = self.size,
            modified_time = self.modified_time,
            checksum = 0,
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
        // FIXME: Due to an off-by-one error somewhere above, the checksum is always too high.
        // For now, manually subtract 64 from the sum to get a valid checksum.
        write!(checksum, "{:06o}\0 ", sum - 64)?;

        &mut vec_writer[148..156].swap_with_slice(&mut checksum[0..]);
        writer.write_all(&vec_writer)?;

        // Header is exactly 12 bytes shy of a single block.
        // Write 12 nulls to fill the block before moving on.
        write!(writer, "{:\0<size$}", "", size = 12)
    }
}

bitflags! {
    struct Mode: u64 {
        const SET_UID = 0o04000;
        const SET_GID = 0o02000;
        const READ_BY_OWNER = 0o00400;
        const WRITE_BY_OWNER = 0o00200;
        const READ_BY_GROUP = 0o00040;
        const WRITE_BY_GROUP = 0o00020;
        const READ_BY_OTHER = 0o00004;
        const WRITE_BY_OTHER = 0o00002;
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
enum TypeFlag {
    RegFile = b'0',
    ARegFile = b'\0',
    Link = b'1',
    Directory = b'5',
}
