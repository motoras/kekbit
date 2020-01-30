pub mod reader;
use reader::ShmReader;
pub mod writer;
use writer::ShmWriter;

use crate::header::{write_header, HEADER_LEN};
use crate::tick::TickUnit;
use crate::utils::{align, compute_max_msg_len, MIN_CAPACITY};
use log::{error, info};
use memmap::MmapOptions;
use std::cmp::max;
use std::cmp::min;
use std::fs::OpenOptions;
use std::fs::{remove_file, DirBuilder};
use std::ops::Shl;
use std::path::Path;
use std::result::Result;

pub fn shm_reader(
    root_path: &Path,
    producer_id: u64,
    channel_id: u64,
    file_id: u64,
) -> Result<ShmReader, String> {
    let dir_path = root_path
        .join(channel_id.to_string())
        .join(producer_id.to_string());
    let kek_file_name = dir_path.join(format!("{}.kekbit", file_id));
    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(&kek_file_name)
        .unwrap();
    info!("Kekbit file {:?} opened for read.", kek_file);
    let mmap = unsafe { MmapOptions::new().map_mut(&kek_file).unwrap() };
    ShmReader::new(mmap)
}

pub fn shm_writer(
    root_path: &Path,
    producer_id: u64,
    channel_id: u64,
    file_id: u64,
    len: u32,
    timeout: u64,
    tick_unit: TickUnit,
) -> Result<ShmWriter, String> {
    let dir_path = root_path
        .join(channel_id.to_string())
        .join(producer_id.to_string());
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(&dir_path).unwrap();
    let lock_file_name = dir_path.join(format!("{}.kekbit.lock", file_id));
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_file_name)
        .unwrap();
    info!("Kekbit lock {:?} created", lock_file_name);
    let kek_file_name = dir_path.join(format!("{}.kekbit", file_id));
    let kek_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&kek_file_name)
        .unwrap();
    let capacity = max(MIN_CAPACITY, align(min(len, 1u32.shl(31))) as usize);
    let total_len = (capacity + HEADER_LEN) as u64;
    kek_file.set_len(total_len).unwrap();
    info!("Kekbit file {:?} created", kek_file);
    let max_msg_len = compute_max_msg_len(capacity as u32);
    let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file).unwrap() };
    let buf = &mut mmap[..];
    write_header(
        buf,
        producer_id,
        channel_id,
        capacity as u32,
        max_msg_len,
        timeout,
        tick_unit,
    );
    mmap.flush().unwrap();
    info!("Kekbit file {:?} initialized", kek_file_name);
    let res = ShmWriter::new(mmap);
    if res.is_err() {
        error!(
            "Kekbit writer creation error . The file {:?} will be removed!",
            kek_file_name
        );
        remove_file(&kek_file_name).expect("Could not remove kekbit file");
    }
    remove_file(&lock_file_name).expect("Could not remove kekbit lock file");
    info!("Kekbit lock file {:?} removed", lock_file_name);
    res
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::{align, REC_HEADER_LEN};
    use std::sync::Once;

    static INIT_LOG: Once = Once::new();

    #[test]
    fn read_than_write() {
        INIT_LOG.call_once(|| {
            simple_logger::init().unwrap();
        });
        let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
        let mut writer = shm_writer(
            &test_tmp_dir.path(),
            100,
            100,
            1,
            10000,
            99999999,
            TickUnit::Nanos,
        )
        .unwrap();
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
        let mut reader = shm_reader(&test_tmp_dir.path(), 100, 100, 1).unwrap();
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
