//! Defines operations to create readers and writers backed by a memory mapped channel.
pub mod reader;
use reader::ShmReader;
pub mod writer;
use crate::header::Header;
use log::{error, info};
use memmap::MmapOptions;

use crate::api::ChannelError;
use crate::api::ChannelError::*;
use std::fs::OpenOptions;
use std::fs::{remove_file, DirBuilder};
use std::path::Path;
use std::result::Result;
use writer::ShmWriter;
/// Creates a kekbit reader associated to a memory mapped channel.
///
/// Returns a ready to use reader which points to the beginning of a kekbit channel if succeeds, or an error if the operation fails.
///
/// # Arguments
///
/// * `root_path` - The path to the folder where all the channels will be stored grouped by writer's id.
/// * `writer_id` - The id of the writer which created the channel.
/// * `channel_id` - The channel identifier.
///
/// # Errors
///
/// Various [errors](enum.ChannelError.html) may occur if the operation fails.
///
/// # Examples
///
/// ```
/// # use kekbit_core::tick::TickUnit::Nanos;
/// # use kekbit_core::header::Header;
/// use kekbit_core::shm::*;
/// # const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// let reader = shm_reader(&test_tmp_dir.path(), channel_id).unwrap();
/// println!("{:?}", reader.header());
///
/// ```
pub fn shm_reader(root_path: &Path, channel_id: u64) -> Result<ShmReader, ChannelError> {
    let kek_file_path = storage_path(root_path, channel_id).into_path_buf();
    let kek_lock_path = kek_file_path.with_extension("lock");
    if !kek_file_path.exists() {
        return Err(StorageNotFound {
            file_name: kek_file_path.to_str().unwrap().to_string(),
        });
    }
    if kek_lock_path.exists() {
        return Err(StorageNotReady {
            file_name: kek_file_path.to_str().unwrap().to_string(),
        });
    }

    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(&kek_file_path)
        .or_else(|err| {
            Err(CouldNotAccessStorage {
                file_name: err.to_string(),
            })
        })?;

    info!("Kekbit file {:?} opened for read.", kek_file);
    let mmap =
        unsafe { MmapOptions::new().map_mut(&kek_file) }.or_else(|err| Err(MemoryMappingFailed { reason: err.to_string() }))?;
    ShmReader::new(mmap)
}

/// Tries multiple times to create a kekbit reader associated to a memory mapped channel.
/// This function will basically call [shm_reader](fn.shm_reader.html) up to *tries* time unless
/// it succeeds. Between two tries the function will spin/sleep for a about ```duration_millis/tries```
/// milliseconds so potentially could be blocking.
/// This should be the preferred method to create a *reader* when you are willing to wait until the channel is available.
///
///
/// Returns a ready to use reader which points to the beginning of a kekbit channel if succeeds, or the error *returned by the last try* if it fails.
///
/// # Arguments
///
/// * `root_path` - The path to the folder where all the channels will be stored grouped by writer's id.
/// * `writer_id` - The id of the writer which created the channel.
/// * `channel_id` - The channel identifier.
/// * `duration_millis` - How long it should try in milliseconds
/// * `tries` - How many times it will try during the given time duration
///
/// # Errors
///
/// Various [errors](enum.ChannelError.html) may occur if the operation fails.
///
/// # Examples
///
/// ```
/// # use kekbit_core::tick::TickUnit::Nanos;
/// # use kekbit_core::header::Header;
/// use kekbit_core::shm::*;
/// # const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// let duration = 1000;
/// let tries = 10;
/// let reader = try_shm_reader(&test_tmp_dir.path(), channel_id, duration, tries).unwrap();
/// println!("{:?}", reader.header());
///
/// ```
pub fn try_shm_reader(root_path: &Path, channel_id: u64, duration_millis: u64, tries: u64) -> Result<ShmReader, ChannelError> {
    assert!(tries > 0);
    let interval = duration_millis / tries;
    let sleep_duration = std::time::Duration::from_millis(interval);
    let mut reader_res = shm_reader(root_path, channel_id);
    let mut tries_left = tries - 1;
    while reader_res.is_err() && tries_left > 0 {
        std::thread::sleep(sleep_duration);
        reader_res = shm_reader(root_path, channel_id);
        tries_left -= 1;
    }
    reader_res
}

/// Creates a file backed memory mapped  kekbit channel and a writer associate with it.
///
/// Returns a ready to use writer to the new created channel or an error if the operation fails.
///
/// # Arguments
///
/// * `root_path` - The path to the folder where all the channels will be stored grouped by writers id.
/// * `header` - a structure of type [Header](struct.Header.html) which contains the complete information required to create a channel.
///
/// # Errors
///
/// Various [errors](enum.ChannelError.html) may occur if the operation fails.
///
/// # Examples
///
/// ```
/// use kekbit_core::tick::TickUnit::Nanos;
/// use kekbit_core::shm::*;
/// use kekbit_core::header::Header;
/// use kekbit_core::api::Writer;
///
/// const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// let capacity = 3000;
/// let max_msg_len = 100;
/// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// writer.heartbeat().unwrap();
/// ```
pub fn shm_writer(root_path: &Path, header: &Header) -> Result<ShmWriter, ChannelError> {
    let kek_file_path = storage_path(root_path, header.channel_id()).into_path_buf();
    if kek_file_path.exists() {
        return Err(StorageAlreadyExists {
            file_name: kek_file_path.to_str().unwrap().to_string(),
        });
    }
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(&kek_file_path.parent().unwrap()).or_else(|err| {
        Err(CouldNotAccessStorage {
            file_name: err.to_string(),
        })
    })?;
    let kek_lock_path = kek_file_path.with_extension("lock");
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&kek_lock_path)
        .or_else(|err| {
            Err(CouldNotAccessStorage {
                file_name: err.to_string(),
            })
        })?;
    info!("Kekbit lock {:?} created", kek_lock_path);
    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&kek_file_path)
        .or_else(|err| {
            Err(CouldNotAccessStorage {
                file_name: err.to_string(),
            })
        })?;
    let total_len = (header.capacity() + header.len() as u32) as u64;
    kek_file.set_len(total_len).or_else(|err| {
        Err(CouldNotAccessStorage {
            file_name: err.to_string(),
        })
    })?;
    info!("Kekbit channel store {:?} created.", kek_file);
    let mut mmap =
        unsafe { MmapOptions::new().map_mut(&kek_file) }.or_else(|err| Err(MemoryMappingFailed { reason: err.to_string() }))?;
    let buf = &mut mmap[..];
    header.write_to(buf);
    mmap.flush().or_else(|err| Err(AccessError { reason: err.to_string() }))?;
    info!("Kekbit channel with store {:?} succesfully initialized", kek_file_path);
    let res = ShmWriter::new(mmap);
    if res.is_err() {
        error!("Kekbit writer creation error . The file {:?} will be removed!", kek_file_path);
        remove_file(&kek_file_path).expect("Could not remove kekbit file");
    }
    remove_file(&kek_lock_path).expect("Could not remove kekbit lock file");
    info!("Kekbit lock file {:?} removed", kek_lock_path);
    res
}

#[inline]
/// Returns the path to the file associated with a channel inside a kekbit root folder.
///
/// # Arguments
///
///  * `root_path` - Path to the kekbit root folder, a folder where channels are stored. Multiple such
///   folders may exist in a system.  
///  * `channel_id` - Channel for which the file path will be returned
///
pub fn storage_path(root_path: &Path, channel_id: u64) -> Box<Path> {
    let high_val: u32 = (channel_id >> 32) as u32;
    let low_val = (channel_id & 0x0000_0000_FFFF_FFFF) as u32;
    let channel_folder = format!("{:04x}_{:04x}", high_val >> 16, high_val & 0x0000_FFFF);
    let channel_file = format!("{:04x}_{:04x}", low_val >> 16, low_val & 0x0000_FFFF);
    let dir_path = root_path.join(channel_folder).join(channel_file);
    dir_path.with_extension("kekbit").into_boxed_path()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::{InvalidPosition, Reader, Writer};
    use crate::tick::TickUnit::Nanos;
    use crate::utils::{align, REC_HEADER_LEN};
    use assert_matches::*;
    use std::sync::Arc;
    use std::sync::Once;
    use tempdir::TempDir;

    const FOREVER: u64 = 99_999_999_999;

    static INIT_LOG: Once = Once::new();

    #[test]
    fn check_max_len() {
        let header = Header::new(100, 1000, 300_000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        assert_eq!(writer.header(), reader.header());
    }

    #[test]
    fn read_than_write() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        let mut msg_count = 0;
        let mut bytes_written = 8; //account for the initial heartbeat
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            let size = writer.write(&to_wr, len).unwrap();
            assert_eq!(size, align(len + REC_HEADER_LEN));
            bytes_written += size;
            msg_count += 1;
        }
        assert_eq!(writer.write_offset(), bytes_written);
        writer.flush().unwrap(); //not really necessary
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        assert_eq!(reader.position(), 0);
        let mut res_msg = StrMsgsAppender::default();
        let bytes_read = reader
            .read(&mut |_pos, msg| res_msg.on_message(msg), msg_count + 10 as u16)
            .unwrap();
        assert_eq!(res_msg.txt, txt);
        assert_eq!(bytes_written, bytes_read);
        assert_eq!(reader.position(), bytes_read);
    }

    #[derive(Default, Debug)]
    struct StrMsgsAppender {
        txt: String,
    }

    impl StrMsgsAppender {
        pub fn on_message(&mut self, buf: &[u8]) {
            let msg_str = std::str::from_utf8(&buf).unwrap();
            if !self.txt.is_empty() {
                self.txt.push_str(" ");
            }
            self.txt.push_str(msg_str);
        }
    }

    #[test]
    fn check_position() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        let mut msg_count = 0;
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            let size = writer.write(&to_wr, len).unwrap();
            assert_eq!(size, align(len + REC_HEADER_LEN));
            msg_count += 1;
        }
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        let mut read_bytes = 0;
        let mut last_msg_size = 0;
        for _i in 0..msg_count {
            last_msg_size = reader.read(&mut |pos, _msg| assert_eq!(pos, read_bytes), 1).unwrap();
            read_bytes += last_msg_size;
        }
        assert_eq!(reader.position(), writer.write_offset() - last_msg_size);
    }

    #[test]
    fn check_move_to() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        let mut msg_count = 0;
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            let size = writer.write(&to_wr, len).unwrap();
            assert_eq!(size, align(len + REC_HEADER_LEN));
            msg_count += 1;
        }
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        reader.move_to(8).unwrap(); //skip  heartbeat
        let mut msg_read = 0;
        let mut last_pos = 0;
        for _i in 0..msg_count {
            //let's read every message twice...
            reader
                .read(
                    &mut |pos, _| {
                        msg_read += 1;
                        last_pos = pos
                    },
                    1,
                )
                .unwrap();
            reader.move_to(last_pos).unwrap();
            reader.read(&mut |_, _| msg_read += 1, 1).unwrap();
        }
        assert_eq!(msg_read, 2 * msg_count);
        reader.move_to(8).unwrap(); //now let's read them again
        for _i in 0..msg_count {
            reader
                .read(
                    &mut |_, _| {
                        msg_read += 1;
                    },
                    1,
                )
                .unwrap();
        }
        assert_eq!(msg_read, 3 * msg_count);
    }

    #[test]
    fn check_invalid_move_to() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            writer.write(&to_wr, len).unwrap();
        }
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        reader.move_to(8).unwrap(); //skip  heartbeat
        assert_matches!(reader.move_to(4), Err(InvalidPosition::Unaligned { position: 4 }));
        assert_matches!(reader.move_to(45680), Err(InvalidPosition::Unavailable { position: 45680 })); //to big
        assert_matches!(reader.move_to(999), Err(InvalidPosition::Unaligned { position: 999 })); //unaligned
        assert!(reader.move_to(24).is_ok());
        assert!(reader.move_to(56).is_ok());
        assert_matches!(reader.move_to(64), Err(InvalidPosition::Unaligned { position: 64 })); //between records
        assert!(reader.move_to(72).is_ok());
        assert_matches!(reader.move_to(832), Err(InvalidPosition::Unavailable { position: 832 }));
        //to big
    }

    #[test]
    fn check_path_to_storage() {
        let dir = TempDir::new("kektest").unwrap();
        let root_path = dir.path();
        let channel_id_0: u64 = 0;
        let path = storage_path(root_path, channel_id_0).into_path_buf();
        assert_eq!(path, root_path.join("0000_0000").join("0000_0000.kekbit"));
        assert_eq!(
            path.with_extension("lock"),
            root_path.join("0000_0000").join("0000_0000.lock")
        );

        let channel_id_1: u64 = 0xAAAA_BBBB_CCCC_DDDD;
        let path = storage_path(root_path, channel_id_1).into_path_buf();
        assert_eq!(path, root_path.join("aaaa_bbbb").join("cccc_dddd.kekbit"));
        assert_eq!(
            path.with_extension("lock"),
            root_path.join("aaaa_bbbb").join("cccc_dddd.lock")
        );
        let channel_id_2: u64 = 0xBBBB_CCCC_0001;
        let path = storage_path(root_path, channel_id_2).into_path_buf();
        assert_eq!(path, root_path.join("0000_bbbb").join("cccc_0001.kekbit"));
        assert_eq!(
            path.with_extension("lock"),
            root_path.join("0000_bbbb").join("cccc_0001.lock")
        );
        let channel_id_3: u64 = 0xAAAA_00BB_000C_0DDD;
        let path = storage_path(root_path, channel_id_3).into_path_buf();
        assert_eq!(path, root_path.join("aaaa_00bb").join("000c_0ddd.kekbit"));
        assert_eq!(
            path.with_extension("lock"),
            root_path.join("aaaa_00bb").join("000c_0ddd.lock")
        );
    }

    #[test]
    fn try_to_create_reader() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let test_tmp_dir = Arc::new(TempDir::new("kektest").unwrap());
        let never_reader = try_shm_reader(&test_tmp_dir.path(), 999_999, 300, 30);
        assert!(never_reader.is_err());
        let channel_id = 999;
        let root_dir = test_tmp_dir.clone();
        let handle = std::thread::spawn(move || {
            let good_reader = try_shm_reader(&test_tmp_dir.path(), channel_id, 1000, 20);
            assert!(good_reader.is_err());
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        shm_writer(&root_dir.path(), &header).unwrap();
        handle.join().unwrap();
    }
}
