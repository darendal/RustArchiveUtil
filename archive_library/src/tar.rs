use std::fs::{read, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::macos::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
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

        let mut result_path = path.clone();
        result_path.set_extension("tar");
        let mut writer = BufWriter::new(File::create(result_path).unwrap());

        record.write_record(&mut writer);

        // write 2 empty blocks to signify end of TAR
        write!(writer, "{:\0<size$}", "", size = BLOCK_SIZE * 2);

        Tar { files: Vec::new() }
    }
}

#[derive(Debug)]
struct TarRecord {
    name: String,
    mode: Mode,
    user_id: u64,
    group_id: u64,
    size: u64,          // size of the file in bytes
    modified_time: i64, // Unix time file modified
    type_flag: TypeFlag,
    linkname: String,
    username: String,
    group_name: String,
    prefix: Vec<char>,
    file: File,
}

impl TarRecord {
    pub fn new(path: PathBuf) -> TarRecord {
        println!("making record for file {:?}", path);
        let name = path.file_name().unwrap();

        let file = File::open(path.clone()).unwrap();
        let metadata = file.metadata().unwrap();

        let mode = Mode::WriteByOwner;
        let user_id = metadata.st_uid();
        let group_id = metadata.st_gid();
        let size = metadata.len();
        let modified_time = metadata.st_mtime();
        let checksum = 0u64;
        let type_flag = TypeFlag::ARegFile;

        let username = get_user_by_uid(user_id).unwrap();
        let group_name = get_group_by_gid(group_id).unwrap();

        TarRecord {
            name: name.to_str().unwrap().to_string(),
            mode,
            user_id: user_id as u64,
            group_id: group_id as u64,
            size,
            modified_time,
            type_flag,
            linkname: "".to_string(),
            username: username.name().to_str().unwrap().to_string(),
            group_name: group_name.name().to_str().unwrap().to_string(),
            prefix: vec![],
            file,
        }
    }

    pub fn write_record(&self, writer: &mut impl Write) {
        self.write_header(writer);

        let mut reader = BufReader::new(&self.file);

        loop {
            let buf = reader.fill_buf().unwrap();
            let len = buf.len();
            if buf.len() == 0 {
                break;
            }
            writer.write_all(buf);

            reader.consume(len)
        }

        let residual = BLOCK_SIZE - (self.size as usize % BLOCK_SIZE);

        if residual != BLOCK_SIZE {
            write!(writer, "{:\0<size$}", "", size = residual);
        }
    }

    fn write_header(&self, writer: &mut impl Write) {

        let mut vec_writer : Vec<u8> = Vec::new();

        // Write all elements of the header to the vector
        write!(vec_writer, "{:\0<100}", self.name);
        write!(vec_writer, "{:06o} \0", 420u64);
        write!(vec_writer, "{:06o} \0", self.user_id);
        write!(vec_writer, "{:06o} \0", self.group_id);
        write!(vec_writer, "{:011o} ", self.size);
        write!(vec_writer, "{:011o} ", self.modified_time);
        // Set checksum to 0 before calculating it.
        write!(vec_writer, "{:06o}\0 ", 0);
        write!(vec_writer, "{}", self.type_flag as u8);
        write!(vec_writer, "{:\0<100}", self.linkname);
        write!(vec_writer, "{:\0<6}", TAR_MAGIC);
        write!(vec_writer, "{:02}", TAR_VERSION);
        write!(vec_writer, "{:\0<32}", self.username);
        write!(vec_writer, "{:\0<32}", self.group_name);
        write!(vec_writer, "{:06o} \0", DEV_MAJOR_VERSION);
        write!(vec_writer, "{:06o} \0", DEV_MINOR_VERSION);
        write!(vec_writer, "{:\0<size$}", "", size = PREFIX_SIZE);

        let sum: u64 = vec_writer.iter().map(|&x| x as u64).sum();

        println!("{:06o}", sum - 64);

        let mut checksum: Vec<u8> = Vec::new();

        write!(checksum, "{:06o}\0 ", sum - 64);

        let mut slice1 = &mut vec_writer[148..156];

        slice1.swap_with_slice(&mut checksum[0..8]);

        writer.write_all(&vec_writer);

        // Header is exactly 12 bytes shy of a single block.
        // Write 12 nulls to fill the block before moving on.
        write!(writer, "{:\0<size$}", "", size = 12);
    }
}

#[repr(u64)]
#[derive(Debug, Copy, Clone)]
enum Mode {
    SetUid = 0o04000,
    SetGid = 0o02000,
    ReadByOwner = 0o00400,
    WriteByOwner = 0o00200,
    ReadByGroup = 0o00040,
    WriteByGroup = 0o00020,
    ReadByOther = 0o00002,
    WriteByOther = 0o00001,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum TypeFlag {
    RegFile = b'0',
    ARegFile = b'\0',
    Link = b'1',
    Directory = b'5',
}
