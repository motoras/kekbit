//! Defines operations to create readers and writers backed by a memory mapped channel.
pub mod reader;
use reader::ShmReader;
pub mod writer;
use crate::header::Header;
use log::{error, info};
use memmap::MmapOptions;

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
///Various errors may occur if the file associated to the channel cannot be created, or the channel metadata is corrupted.
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
/// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, 1, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// let reader = shm_reader(&test_tmp_dir.path(), writer_id, channel_id).unwrap();
/// println!("{:?}", reader.header());
///
/// ```
pub fn shm_reader(root_path: &Path, writer_id: u64, channel_id: u64) -> Result<ShmReader, String> {
    let dir_path = root_path.join(writer_id.to_string());
    let kek_file_name = dir_path.join(format!("{}.kekbit", channel_id));
    let kek_lock_name = dir_path.join(format!("{}.kekbit.lock", channel_id));
    if !kek_file_name.exists() {
        return Err(format!("{:?} does not exist.", kek_file_name));
    }
    if kek_lock_name.exists() {
        return Err(format!("{:?} Is not ready yet", kek_file_name));
    }

    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(&kek_file_name)
        .or_else(|err| Err(err.to_string()))?;

    info!("Kekbit file {:?} opened for read.", kek_file);
    let mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.or_else(|err| Err(err.to_string()))?;
    ShmReader::new(mmap)
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
///Various errors may occur if the file associated with the channel cannot be created, already exists or the header is not correct specified
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
/// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos.nix_time(), Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// writer.heartbeat().unwrap();
/// ```
pub fn shm_writer(root_path: &Path, header: &Header) -> Result<ShmWriter, String> {
    let dir_path = root_path.join(header.producer_id().to_string());
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(&dir_path).or_else(|err| Err(err.to_string()))?;
    let lock_file_name = dir_path.join(format!("{}.kekbit.lock", header.channel_id()));
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_file_name)
        .or_else(|err| Err(err.to_string()))?;
    info!("Kekbit lock {:?} created", lock_file_name);
    let kek_file_name = dir_path.join(format!("{}.kekbit", header.channel_id()));
    if kek_file_name.exists() {
        error!(
            "Kekbit writer creation error . The channel file {:?} already exists",
            kek_file_name
        );
        return Err("Channel file already exists!".to_string());
    }
    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&kek_file_name)
        .or_else(|err| Err(err.to_string()))?;
    let total_len = (header.capacity() + header.len() as u32) as u64;
    kek_file.set_len(total_len).or_else(|err| Err(err.to_string()))?;
    info!("Kekbit channel store {:?} created.", kek_file);
    let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.or_else(|err| Err(err.to_string()))?;
    let buf = &mut mmap[..];
    header.write_to(buf);
    mmap.flush().or_else(|err| Err(err.to_string()))?;
    info!("Kekbit channel with store {:?} succesfully initialized", kek_file_name);
    let res = ShmWriter::new(mmap);
    if res.is_err() {
        error!("Kekbit writer creation error . The file {:?} will be removed!", kek_file_name);
        remove_file(&kek_file_name).expect("Could not remove kekbit file");
    }
    remove_file(&lock_file_name).expect("Could not remove kekbit lock file");
    info!("Kekbit lock file {:?} removed", lock_file_name);
    res
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::{Reader, Writer};
    use crate::tick::TickUnit::Nanos;
    use crate::utils::{align, REC_HEADER_LEN};
    use std::sync::Once;

    const FOREVER: u64 = 99_999_999_999;

    static INIT_LOG: Once = Once::new();

    #[test]
    fn check_max_len() {
        let header = Header::new(100, 1000, 300_000, 1000, FOREVER, 1, Nanos);
        let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
        let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let reader = shm_reader(&test_tmp_dir.path(), 100, 1000).unwrap();
        assert_eq!(writer.header(), reader.header());
    }

    #[test]
    fn read_than_write() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let header = Header::new(100, 1000, 10000, 1000, FOREVER, 1000, Nanos);
        let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
        let txt = "There are 10 kinds of people: those who know binary and those who don't";
        let msgs = txt.split_whitespace();
        let mut msg_count = 0;
        let mut bytes_written = 8; //accout for the initial heartbeat
        for m in msgs {
            let to_wr = m.as_bytes();
            let len = to_wr.len() as u32;
            let size = writer.write(&to_wr, len).unwrap();
            assert_eq!(size, align(len + REC_HEADER_LEN));
            bytes_written += size;
            msg_count += 1;
        }
        let mut reader = shm_reader(&test_tmp_dir.path(), 100, 1000).unwrap();
        assert_eq!(reader.total_read(), 0);
        let mut res_msg = StrMsgsAppender::default();
        let bytes_read = reader
            .read(&mut |msg| res_msg.on_message(msg), msg_count + 10 as u16)
            .unwrap();
        assert_eq!(res_msg.txt, txt);
        assert_eq!(bytes_written, bytes_read);
        assert_eq!(reader.total_read(), bytes_read);
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
}
