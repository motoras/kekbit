use crate::api::{WriteError, Writer};
use crate::header::Header;
use crate::utils::{align, store_atomic_u64, CLOSE, REC_HEADER_LEN, WATERMARK};
use log::{debug, error, info};
use memmap::MmapMut;
use std::ptr::copy_nonoverlapping;
use std::result::Result;
use std::sync::atomic::Ordering;

#[derive(Debug)]
pub struct ShmWriter {
    header: Header,
    data_ptr: *mut u8,
    write_offset: u32,
    mmap: MmapMut,
}

impl ShmWriter {
    #[allow(clippy::cast_ptr_alignment)]
    pub(super) fn new(mut mmap: MmapMut) -> Result<ShmWriter, String> {
        let buf = &mut mmap[..];
        let header = Header::read(buf)?;
        let header_ptr = buf.as_ptr() as *mut u64;
        let head_len = header.len();
        let data_ptr = unsafe { header_ptr.add(head_len) } as *mut u8;
        let mut writer = ShmWriter {
            header,
            data_ptr,
            write_offset: 0,
            mmap,
        };
        info!(
            "Kekbit channel writer created. Size is {}MB. Max msg size {}KB",
            writer.header.capacity() / 1_000_000,
            writer.header.max_msg_len() / 1_000
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

    #[inline(always)]
    unsafe fn write_metadata(&mut self, write_ptr: *mut u64, len: u64, aligned_rec_len: u32) {
        store_atomic_u64(write_ptr.add(aligned_rec_len as usize), WATERMARK, Ordering::Release);
        store_atomic_u64(write_ptr, len, Ordering::Release);
    }
}

impl Writer for ShmWriter {
    #[allow(clippy::cast_ptr_alignment)]
    fn write(&mut self, data: &[u8], len: u32) -> Result<u32, WriteError> {
        if len > self.header.max_msg_len() {
            return Err(WriteError::MaxRecordLenExceed {
                rec_len: len,
                max_allowed: self.header.max_msg_len(),
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
            copy_nonoverlapping(data.as_ptr(), write_ptr.add(REC_HEADER_LEN as usize) as *mut u8, len as usize);
            self.write_metadata(write_ptr as *mut u64, len as u64, aligned_rec_len >> 3);
        }
        self.write_offset += aligned_rec_len;
        Ok(aligned_rec_len as u32)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), std::io::Error> {
        debug!("Flushing the channel");
        self.mmap.flush()
    }
}
impl Drop for ShmWriter {
    fn drop(&mut self) {
        let write_index = self.write_offset;
        info!("Closing message queue..");
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let write_ptr = self.data_ptr.offset(write_index as isize) as *mut u64;
            store_atomic_u64(write_ptr, CLOSE, Ordering::Release);
            info!("Closing message sent")
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
        self.header.capacity() - self.write_offset
    }
    #[inline]
    pub fn write_offset(&self) -> u32 {
        self.write_offset
    }

    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
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
