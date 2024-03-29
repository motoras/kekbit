//! Provides the components and functions required to work with memory mapped data channels.
mod handlers;
mod metadata;
mod reader;
mod tick;
mod utils;
mod version;
mod writer;

pub use handlers::*;
pub use metadata::*;
pub use reader::*;
pub use tick::*;
pub use writer::*;

use log::{error, info};
use memmap::MmapOptions;

use crate::api::ChannelError;
use crate::api::ChannelError::*;
use crate::api::Handler;

use crate::core::utils::FOOTER_LEN;
use std::fs::OpenOptions;
use std::fs::{remove_file, DirBuilder};
use std::path::Path;
use std::result::Result;
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
/// Various [errors](../api/enum.ChannelError.html) may occur if the operation fails.
///
/// # Examples
///
/// ```
/// # use kekbit::core::TickUnit::Nanos;
/// use kekbit::core::*;
/// use kekbit::api::*;
/// # const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
/// let reader = shm_reader(&test_tmp_dir.path(), channel_id).unwrap();
/// println!("{:?}", reader.metadata());
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
        .map_err(|err| CouldNotAccessStorage {
            file_name: err.to_string(),
        })?;

    info!("Kekbit file {:?} opened for read.", kek_file);
    let mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.map_err(|err| MemoryMappingFailed { reason: err.to_string() })?;
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
/// # use kekbit::core::TickUnit::Nanos;
/// use kekbit::core::*;
/// use kekbit::api::*;
/// # const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
/// let duration = 1000;
/// let tries = 10;
/// let reader = try_shm_reader(&test_tmp_dir.path(), channel_id, duration, tries).unwrap();
/// println!("{:?}", reader.metadata());
///
/// ```
pub fn try_shm_reader(root_path: &Path, channel_id: u64, duration_millis: u64, tries: u64) -> Result<ShmReader, ChannelError> {
    assert!(tries > 0);
    let interval = duration_millis / tries;
    let sleep_duration = std::time::Duration::from_millis(interval);
    let mut reader_res = shm_reader(root_path, channel_id);
    let mut tries_left = tries;
    while reader_res.is_err() && tries_left > 0 {
        std::thread::sleep(sleep_duration);
        reader_res = shm_reader(root_path, channel_id);
        tries_left -= 1;
    }
    reader_res
}
//This method should be removed as soon as metadata is exposed in the reader trait
/// Decorates a [ShmReader](struct.ShmReader.html) with a timeout functionality.
///
/// # Examples
///
/// ```
/// # use kekbit::core::TickUnit::Millis;
/// use kekbit::core::*;
/// use kekbit::api::*;
/// const TWO_SECS: u64 = 2000;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, TWO_SECS, Millis);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
/// let reader = shm_reader(&test_tmp_dir.path(), channel_id).unwrap();
/// let timeout_reader = shm_timeout_reader(reader);
/// ```
#[inline]
pub fn shm_timeout_reader(reader: ShmReader) -> TimeoutReader<ShmReader> {
    reader.into()
}

/// Creates a file backed memory mapped  kekbit channel and a writer associate with it.
///
/// Returns a ready to use writer to the new created channel or an error if the operation fails.
///
/// # Arguments
///
/// * `root_path` - The path to the folder where all the channels will be stored grouped by writers id.
/// * `metadata` - a structure of type [Metadata](struct.Metadata.html) which contains the complete information required to create a channel.
///
/// # Errors
///
/// Various [errors](enum.ChannelError.html) may occur if the operation fails.
///
/// # Examples
///
/// ```
/// use kekbit::core::TickUnit::Nanos;
/// use kekbit::core::*;
/// use kekbit::api::*;
///
/// const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// let capacity = 3000;
/// let max_msg_len = 100;
/// let metadata = Metadata::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// let mut writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
/// ```
pub fn shm_writer<H: Handler>(root_path: &Path, metadata: &Metadata, rec_handler: H) -> Result<ShmWriter<H>, ChannelError> {
    let kek_file_path = storage_path(root_path, metadata.channel_id()).into_path_buf();
    if kek_file_path.exists() {
        return Err(StorageAlreadyExists {
            file_name: kek_file_path.to_str().unwrap().to_string(),
        });
    }
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder
        .create(&kek_file_path.parent().unwrap())
        .map_err(|err| CouldNotAccessStorage {
            file_name: err.to_string(),
        })?;
    let kek_lock_path = kek_file_path.with_extension("lock");
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&kek_lock_path)
        .map_err(|err| CouldNotAccessStorage {
            file_name: err.to_string(),
        })?;
    info!("Kekbit lock {:?} created", kek_lock_path);
    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&kek_file_path)
        .map_err(|err| CouldNotAccessStorage {
            file_name: err.to_string(),
        })?;
    let total_len = (metadata.capacity() + metadata.len() as u32 + FOOTER_LEN) as u64;
    kek_file.set_len(total_len).map_err(|err| CouldNotAccessStorage {
        file_name: err.to_string(),
    })?;
    info!("Kekbit channel store {:?} created.", kek_file);
    let mut mmap =
        unsafe { MmapOptions::new().map_mut(&kek_file) }.map_err(|err| MemoryMappingFailed { reason: err.to_string() })?;
    let buf = &mut mmap[..];
    metadata.write_to(buf);
    mmap.flush().map_err(|err| AccessError { reason: err.to_string() })?;
    info!("Kekbit channel with store {:?} succesfully initialized", kek_file_path);
    let res = ShmWriter::new(mmap, rec_handler);
    if res.is_err() {
        error!("Kekbit writer creation error . The file {:?} will be removed!", kek_file_path);
        remove_file(&kek_file_path).expect("Could not remove kekbit file");
    }
    remove_file(&kek_lock_path).expect("Could not remove kekbit lock file");
    info!("Kekbit lock file {:?} removed", kek_lock_path);
    res
}

/// Returns the path to the file associated with a channel inside a kekbit root folder.
///
/// # Arguments
///
///  * `root_path` - Path to the kekbit root folder, a folder where channels are stored. Multiple such
///   folders may exist in a system.  
///  * `channel_id` - Channel for which the file path will be returned
///
#[inline]
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
    use super::tick::TickUnit::Nanos;
    use super::utils::{align, REC_HEADER_LEN};
    use super::*;
    use crate::api::EncoderHandler;
    use crate::api::ReadError;
    use crate::api::ReadError::Timeout;
    use crate::api::Reader;
    use crate::api::Writer;
    use crate::core::TickUnit::Millis;
    use simple_logger::SimpleLogger;
    use std::sync::Arc;
    use std::sync::Once;
    use tempdir::TempDir;
    const FOREVER: u64 = 99_999_999_999;
    static INIT_LOG: Once = Once::new();

    #[test]
    fn check_max_len() {
        let metadata = Metadata::new(100, 1000, 300_000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        let reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        assert_eq!(writer.metadata(), reader.metadata());
    }

    #[test]
    fn write_than_read() {
        INIT_LOG.call_once(|| {
            SimpleLogger::new().init().unwrap();
        });
        let metadata = Metadata::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        let mut msg_count = 0;
        let mut bytes_written = 0;
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            let size = writer.write(&to_wr).unwrap();
            assert_eq!(size, align(len + REC_HEADER_LEN));
            bytes_written += size;
            msg_count += 1;
        }
        assert_eq!(writer.write_offset(), bytes_written);
        writer.flush().unwrap(); //not really necessary
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        assert_eq!(reader.position(), 0);
        let mut msg_iter = reader.try_iter();
        let mut res_txt = String::new();
        for read_res in &mut msg_iter {
            match read_res {
                ReadResult::Record(msg) => {
                    let msg_str = std::str::from_utf8(&msg).unwrap();
                    if !res_txt.is_empty() {
                        res_txt.push(' ');
                    }
                    res_txt.push_str(msg_str);
                    msg_count -= 1;
                }
                ReadResult::Nothing => {
                    assert!(msg_count == 0);
                    break;
                }
                ReadResult::Failed(err) => match err {
                    ReadError::Closed => break,
                    _ => {
                        panic!("Unexpected read error {:?}", err);
                    }
                },
            }
        }
        assert_eq!(res_txt, txt);
        assert_eq!(bytes_written, reader.position());
    }

    #[test]
    fn try_iterator_hint_size() {
        INIT_LOG.call_once(|| {
            SimpleLogger::new().init().unwrap();
        });
        let metadata = Metadata::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut msg_count = 0;
        {
            let mut writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
            let txt = "There are 10 kinds of people: those who know binary and those who don't";
            let msgs = txt.split_whitespace();
            for m in msgs {
                let to_wr = m.as_bytes();
                let len = to_wr.len() as u32;
                let size = writer.write(&to_wr).unwrap();
                assert_eq!(size, align(len + REC_HEADER_LEN));
                msg_count += 1;
            }
        }
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        assert!(reader.exhausted().is_none());
        let mut read_iter = reader.try_iter();
        let sh1 = read_iter.size_hint();
        assert_eq!(sh1.0, 0);
        assert!(sh1.1.is_none());
        read_iter.next().unwrap();
        let sh2 = read_iter.size_hint();
        assert_eq!(sh2.0, 0);
        assert!(sh2.1.is_none());
        //consume it
        let mut total = 0;
        for _msg in &mut read_iter {
            total += 1
        }
        assert_eq!(total, msg_count);
        let sh3 = read_iter.size_hint();
        assert_eq!(sh3.0, 0);
        assert!(sh3.1.unwrap() == 0);
        assert!(read_iter.next().is_none());
        assert!(reader.exhausted().is_some());
        assert_eq!(reader.exhausted().unwrap(), ReadError::Closed);
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
            SimpleLogger::new().init().unwrap();
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
        let metadata = Metadata::new(100, 1000, 10000, 1000, FOREVER, Nanos);
        shm_writer(&root_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        handle.join().unwrap();
    }
    use assert_matches::assert_matches;
    #[test]
    fn read_with_timeout() {
        INIT_LOG.call_once(|| {
            SimpleLogger::new().init().unwrap();
        });
        let timeout = 50;
        let metadata = Metadata::new(100, 1000, 10000, 1000, timeout, Millis);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        let txt = "Just a bad day";
        writer.write(&txt.as_bytes()).unwrap();
        let reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        let mut timeout_reader = shm_timeout_reader(reader);
        let mut msg_iter = timeout_reader.try_iter();
        assert_matches!(msg_iter.next(), Some(ReadResult::Record(_)));
        assert_matches!(msg_iter.next(), Some(ReadResult::Nothing));
        let sleep_duration = std::time::Duration::from_millis(timeout + 10);
        std::thread::sleep(sleep_duration);
        assert_matches!(msg_iter.next(), Some(ReadResult::Failed(Timeout(_))));
        assert_matches!(msg_iter.next(), None);
        writer.flush().unwrap(); //not really necessary
    }
}
