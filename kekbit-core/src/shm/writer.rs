use crate::api::WriteError;
use crate::header;
use crate::tick::TickUnit;
use crate::utils::{align, store_atomic_u64, CLOSE, REC_HEADER_LEN, WATERMARK};
use log::{debug, error, info, warn};
use memmap::MmapMut;
use std::ptr::copy_nonoverlapping;
use std::result::Result;
use std::sync::atomic::Ordering;

static HEARTBEAT_MSG: &[u8] = &[];

#[derive(Debug)]
pub struct ShmWriter {
    data_ptr: *mut u8,
    capacity: u32,
    max_msg_len: u32,
    tick_unit: TickUnit,
    write_offset: u32,
    timeout: u64,
    mmap: MmapMut,
}

impl ShmWriter {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn new(mut mmap: MmapMut) -> Result<ShmWriter, String> {
        let buf = &mut mmap[..];
        header::check_header(&buf)?;
        let header_ptr = buf.as_ptr() as *mut u64;
        let capacity = header::capacity(buf);
        let max_msg_len = header::max_msg_len(buf);
        let timeout = header::prod_timeout(buf) * 2;
        let tick_unit = header::tick_unit(buf);
        let data_ptr = unsafe { header_ptr.add(header::HEADER_LEN as usize) } as *mut u8;
        let mut writer = ShmWriter {
            data_ptr,
            capacity,
            max_msg_len,
            tick_unit,
            write_offset: 0,
            timeout,
            mmap,
        };
        info!("Kekbit writer created");
        info!(
            "Kekbit Channel created. Size is {}MB. Max msg size {}kb",
            capacity / 1_000_000,
            max_msg_len / 1_000
        );
        //sent the very first original heart bear
        match writer.heartbeat() {
            Ok(_) => {
                info!("Initial hearbeat succesfully sent!");
                Ok(writer)
            }
            Err(we) => {
                error!("Initial heartbeat failed!. Reason {:?}", we);
                Err(format!("{:?}", we))
            }
        }
    }
}

impl ShmWriter {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn write(&mut self, data: &[u8], len: u32) -> Result<u32, WriteError> {
        if len > self.max_msg_len {
            return Err(WriteError::MaxRecordLenExceed {
                rec_len: len,
                max_allowed: self.max_msg_len,
            });
        }
        let aligned_rec_len = align(len + REC_HEADER_LEN);
        let avl = self.available();
        if aligned_rec_len > avl {
            return Err(WriteError::NoSpaceAvailable {
                required: aligned_rec_len,
                left: avl,
            });
        }
        let write_index = self.write_offset;
        unsafe {
            let write_ptr = self.data_ptr.offset(write_index as isize);
            copy_nonoverlapping(
                data.as_ptr(),
                write_ptr.add(REC_HEADER_LEN as usize) as *mut u8,
                len as usize,
            );
            self.write_metadata(write_ptr as *mut u64, len as u64, aligned_rec_len >> 3);
        }
        self.write_offset += aligned_rec_len;
        Ok(aligned_rec_len as u32)
    }

    #[inline(always)]
    pub fn heartbeat(&mut self) -> Result<u32, WriteError> {
        self.write(HEARTBEAT_MSG, 0)
    }

    #[inline(always)]
    unsafe fn write_metadata(&mut self, write_ptr: *mut u64, len: u64, aligned_rec_len: u32) {
        store_atomic_u64(
            write_ptr.add(aligned_rec_len as usize),
            WATERMARK,
            Ordering::Release,
        );
        store_atomic_u64(write_ptr, len, Ordering::Release);
    }
    #[inline]
    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        debug!("Flushing the channel");
        self.mmap.flush()
    }
}
impl Drop for ShmWriter {
    fn drop(&mut self) {
        //TODO account for the state of the file...
        let buf = &mut self.mmap[..];
        let write_index = self.write_offset;
        info!("Closing message queue..");
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let write_ptr = self.data_ptr.offset(write_index as isize) as *mut u64;
            store_atomic_u64(write_ptr, CLOSE, Ordering::Release);
            info!("Closing message sent")
        }
        if header::set_status(buf, header::Status::Closed(self.tick_unit.nix_time())).is_ok() {
            info!("File succesfully marked as closed")
        } else {
            warn!("Failed to mark file as closed")
        }
        self.write_offset = self.mmap.len() as u32;
        if self.mmap.flush().is_ok() {
            info!("All changes flushed");
        } else {
            error!("Flush Failed");
        }
    }
}
impl ShmWriter {
    #[inline]
    pub fn available(&self) -> u32 {
        self.capacity - self.write_offset
    }

    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    #[inline]
    pub fn write_offset(&self) -> u32 {
        self.write_offset
    }

    #[inline]
    pub fn max_msg_len(&self) -> u32 {
        self.max_msg_len
    }
}

// #[cfg(test)]
// mod tests {
//use super::*;
//use std::path::Path;
//#[test]
// fn it_works() {

// }
// const TEST_PATH: &str = "/dev/shm/kekmio";

// fn test_path() -> &'static Path {
//     &Path::new(TEST_PATH)
// }
//}
