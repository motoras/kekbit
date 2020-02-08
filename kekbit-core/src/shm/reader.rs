use crate::api::{ReadError, Reader};
use crate::header::Header;
use crate::utils::{align, load_atomic_u64, CLOSE, REC_HEADER_LEN, U64_SIZE, WATERMARK};
use log::{error, info, warn};
use memmap::MmapMut;
use std::ops::FnMut;
use std::result::Result;
use std::sync::atomic::Ordering;

const END_OF_TIME: u64 = std::u64::MAX; //this should be good for any time unit including nanos

/// An implementation of the [Reader](trait.Reader.html) which access a persistent channel through
/// memory mapping. A `ShmReader` must be created using the [shm_reader](fn.shm_reader.html) function.
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
pub struct ShmReader {
    header: Header,
    data_ptr: *mut u8,
    read_index: u32,
    expiration: u64,
    _mmap: MmapMut,
}

impl ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    pub(super) fn new(mut mmap: MmapMut) -> Result<ShmReader, String> {
        let buf = &mut mmap[..];
        let header = Header::read(buf)?;
        let header_ptr = buf.as_ptr() as *mut u64;
        let data_ptr = unsafe { header_ptr.add(header.len() as usize) } as *mut u8;
        info!("Kekbit Reader successfully created");
        Ok(ShmReader {
            header,
            data_ptr,
            read_index: 0,
            expiration: END_OF_TIME,
            _mmap: mmap,
        })
    }
    ///Returns a reference to the [Header](struct.Header.html) associated with this channel
    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    ///Returns the `total` amount of bytes read so far by this reader. This amount
    /// includes the bytes from record headers and the one used for record padding.
    pub fn total_read(&self) -> u32 {
        self.read_index
    }
}
impl Reader for ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    ///Reads up to `message_count` messages from the channel and for each message  
    ///calls the given handler. The handler it is `not` called  for `heartbeat` messages.
    ///This operation is non-blocking, if you want to wait for a message to be available, external
    ///wait/spin mechanisms must be used.
    ///
    ///Returns the amount of bytes read together and/or an error. Even if an error occurred
    ///there may have been messages which were correctly read, and  for which the handler was called.
    ///
    /// # Arguments
    ///
    /// * `handler` - The function which will be called every time a valid messages is read from the channel.
    ///                   The message is just binary data, it's up to the handler to interpret it properly.
    /// * `message_count` - The `maximum` number of messages to be read on this call.
    ///
    /// # Errors
    /// Various [errors](enum.ReadError.html) may occurred if a `writer` timeout is detected, end of channel is reached, channel is closed or channel data is corrupted.
    /// However even in such situations, some valid records *may* have been processed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use kekbit_core::tick::TickUnit::Nanos;
    /// # use kekbit_core::header::Header;
    /// use kekbit_core::shm::*;
    /// use crate::kekbit_core::api::Reader;
    /// # const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, 1, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// # let writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
    /// let mut reader = shm_reader(&test_tmp_dir.path(), writer_id, channel_id).unwrap();
    /// reader.read(&mut |buf| println!("{}",std::str::from_utf8(buf).unwrap()), 10).unwrap();  
    ///
    /// ```
    ///
    fn read(&mut self, handler: &mut impl FnMut(&[u8]) -> (), message_count: u16) -> Result<u32, ReadError> {
        let mut msg_read = 0u16;
        let bytes_at_start = self.read_index;
        while msg_read < message_count {
            let crt_index = self.read_index as usize;
            if crt_index + U64_SIZE >= self.header.capacity() as usize {
                return Err(ReadError::ChannelFull {
                    bytes_read: self.read_index - bytes_at_start,
                });
            }
            let rec_len: u64 = unsafe { load_atomic_u64(self.data_ptr.add(crt_index) as *mut u64, Ordering::Acquire) };
            if rec_len <= self.header.max_msg_len() as u64 {
                let rec_size = align(REC_HEADER_LEN + rec_len as u32);
                if crt_index + rec_size as usize >= self.header.capacity() as usize {
                    return Err(ReadError::ChannelFull {
                        bytes_read: self.read_index - bytes_at_start,
                    });
                }
                if rec_len > 0 {
                    //otherwise is a heartbeat
                    handler(unsafe {
                        std::slice::from_raw_parts(self.data_ptr.add(crt_index + REC_HEADER_LEN as usize), rec_len as usize)
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
        } else if self.expiration == END_OF_TIME {
            self.expiration = self.header.tick_unit().nix_time() + self.header.timeout();
        //start the timeout clock
        } else if self.expiration <= self.header.tick_unit().nix_time() {
            warn!("Writer timeout detected. Channel will be abandoned. No more reads will be performed");
            return Err(ReadError::Timeout {
                timeout: self.expiration,
            });
        }
        Ok(self.read_index - bytes_at_start)
    }
}
