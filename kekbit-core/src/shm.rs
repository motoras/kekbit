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

pub fn shm_reader(root_path: &Path, producer_id: u64, channel_id: u64) -> Result<ShmReader, String> {
    let dir_path = root_path.join(producer_id.to_string());
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

    static INIT_LOG: Once = Once::new();

    #[test]
    fn check_max_len() {
        let header = Header::new(100, 1000, 300_000, 1000, 99999999999, 1, Nanos);
        dbg!(&header);
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
        let header = Header::new(100, 1000, 10000, 1000, 99999999, 1000, Nanos);
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
        let mut res_msg = StrMsgsAppender::default();
        let bytes_read = reader
            .read(&mut |msg| res_msg.on_message(msg), msg_count + 10 as u16)
            .unwrap();
        assert_eq!(res_msg.txt, txt);
        assert_eq!(bytes_written, bytes_read);
    }

    #[derive(Default, Debug)]
    struct StrMsgsAppender {
        txt: String,
    }

    impl StrMsgsAppender {
        pub fn on_message(&mut self, buf: &[u8]) {
            let msg_str = std::str::from_utf8(&buf).unwrap();
            if self.txt.len() > 0 {
                self.txt.push_str(" ");
            }
            self.txt.push_str(msg_str);
        }
    }
}
