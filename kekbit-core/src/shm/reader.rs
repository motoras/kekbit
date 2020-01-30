use crate::api::ReadError;
use crate::header;
use crate::tick::TickUnit;
use crate::utils::{align, load_atomic_u64, CLOSE, REC_HEADER_LEN, U64_SIZE, WATERMARK};
use log::{error, info, warn};
use memmap::MmapMut;
use std::ops::FnMut;
use std::result::Result;
use std::sync::atomic::Ordering;

const END_OF_TIME: u64 = std::u64::MAX; //this should be good for any time unit including nanos

pub struct ShmReader {
    capacity: u32,
    max_msg_len: u64,
    timeout: u64,
    tick_unit: TickUnit,
    data_ptr: *mut u8,
    read_index: u32,
    expiration: u64,
    _mmap: MmapMut,
}

impl ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn new(mut mmap: MmapMut) -> Result<ShmReader, String> {
        let buf = &mut mmap[..];
        header::check_header(&buf)?;
        let capacity = header::capacity(buf);
        let timeout = header::prod_timeout(buf);
        let tick_unit = header::tick_unit(buf);
        let max_msg_len = header::max_msg_len(buf) as u64;
        let header_ptr = buf.as_ptr() as *mut u64;
        let data_ptr = unsafe { header_ptr.add(header::HEADER_LEN as usize) } as *mut u8;
        info!("Kekbit Reader succesfully created");
        Ok(ShmReader {
            capacity,
            max_msg_len,
            timeout,
            tick_unit,
            data_ptr,
            read_index: 0,
            expiration: END_OF_TIME,
            _mmap: mmap,
        })
    }

    #[allow(clippy::cast_ptr_alignment)]
    pub fn read(
        &mut self,
        handler: &mut impl FnMut(&[u8]) -> (),
        message_count: u16,
    ) -> Result<u32, ReadError> {
        let mut msg_read = 0u16;
        let bytes_at_start = self.read_index;
        while msg_read < message_count {
            let crt_index = self.read_index as usize;
            if crt_index + U64_SIZE >= self.capacity as usize {
                return Err(ReadError::EndOfChannel {
                    bytes_read: self.read_index - bytes_at_start,
                });
            }
            let rec_len: u64 = unsafe {
                load_atomic_u64(self.data_ptr.add(crt_index) as *mut u64, Ordering::Acquire)
            };
            if rec_len <= self.max_msg_len {
                let rec_size = align(REC_HEADER_LEN + rec_len as u32);
                if crt_index + rec_size as usize >= self.capacity as usize {
                    return Err(ReadError::EndOfChannel {
                        bytes_read: self.read_index - bytes_at_start,
                    });
                }
                if rec_len > 0 {
                    //otherwise is a heartbeat
                    handler(unsafe {
                        std::slice::from_raw_parts(
                            self.data_ptr.add(crt_index + REC_HEADER_LEN as usize),
                            rec_len as usize,
                        )
                    });
                }
                msg_read += 1;
                self.read_index += rec_size;
            } else {
                match rec_len {
                    WATERMARK => {
                        break;
                    }
                    CLOSE => {
                        info!("Producer closed channel");
                        return Err(ReadError::Closed {
                            bytes_read: self.read_index - bytes_at_start,
                        });
                    }
                    _ => {
                        error!(
                            "Channel corrupted. Unknown Marker {:#016X} at position {} ",
                            rec_len, self.read_index,
                        );
                        return Err(ReadError::Failed {
                            bytes_read: self.read_index - bytes_at_start,
                        });
                    }
                }
            }
        }
        if msg_read > 0 {
            self.expiration = END_OF_TIME;
        } else {
            if self.expiration == END_OF_TIME {
                //start the timeout clock
                self.expiration = self.tick_unit.nix_time() + self.timeout;
            } else {
                if self.expiration <= self.tick_unit.nix_time() {
                    warn!(
                        "Writer timeout detected. Channel will be abnadoned. No more reads will be performed"
                    );
                    return Err(ReadError::Timeout {
                        bytes_read: self.read_index - bytes_at_start,
                        timeout: self.expiration,
                    });
                }
            }
        }
        Ok(self.read_index - bytes_at_start)
    }
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn total_read(&self) -> u32 {
        self.read_index
    }
}
