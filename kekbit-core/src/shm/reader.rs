use crate::api::{ChannelError, ReadError, Reader};
use crate::header::Header;
use crate::utils::{align, load_atomic_u64, CLOSE, REC_HEADER_LEN, U64_SIZE, WATERMARK};
use log::{error, info, warn};
use memmap::MmapMut;
use std::iter::Iterator;
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
/// # use kekbit_codecs::codecs::raw::RawBinDataFormat;
/// use kekbit_core::shm::*;
/// # const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// # let writer = shm_writer(&test_tmp_dir.path(), &header, RawBinDataFormat).unwrap();
/// let reader = shm_reader(&test_tmp_dir.path(), channel_id).unwrap();
/// println!("{:?}", reader.header());
///
/// ```
#[derive(Debug)]
pub struct ShmReader {
    header: Header,
    data_ptr: *const u8,
    read_index: u32,
    expiration: u64,
    _mmap: MmapMut,
}

impl ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    pub(super) fn new(mut mmap: MmapMut) -> Result<ShmReader, ChannelError> {
        let buf = &mut mmap[..];
        let header = Header::read(buf)?;
        let header_ptr = buf.as_ptr() as *mut u64;
        let data_ptr = unsafe { header_ptr.add(header.len() as usize) } as *const u8;
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
    ///Returns the current read position. It is also the `total` amount of bytes read
    ///so far(including bytes from record headers and the one used for record padding)
    pub fn position(&self) -> u32 {
        self.read_index
    }

    /// Returns A non-blocking iterator over messages in the channel.
    ///
    /// Each call to [`next`] returns a message if there is one ready available. The iterator
    /// will never block waiting for a message to be available.
    ///
    /// [`next`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#tymethod.next
    ///
    /// A non-blocking iterator is the most convenient method to read from a channel. The `size_hint` method could
    /// be used to find if new records will ever be available to read from the channel.
    ///
    /// #Example
    ///
    /// let channel_id = 77977;
    /// let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    /// let mut reader = try_shm_reader(&tmp_dir, channel_id, 20_000, 60).unwrap();
    /// let mut stop = false;
    /// while !stop {
    ///    let mut msg_iter = reader.try_iter();
    ///    for msg in &mut msg_iter {
    ///        let msg_str = std::str::from_utf8(&msg).unwrap();
    ///        println!("Got message {}", msg_str);
    ///    }
    ///    if msg_iter.size_hint().1 == Some(0) {
    ///        println!("Nothing more to read. Will stop");
    ///        stop = true;
    ///    } else {
    ///        //wait, spin or do other work....
    ///        std::thread::sleep(std::time::Duration::from_millis(200));
    ///    }
    ///}
    pub fn try_iter(&mut self) -> TryIter {
        TryIter {
            inner: self,
            available: true,
        }
    }
}
impl Reader for ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    ///Attempts to read a message from the channel without blocking.
    ///This method will either read a message from the channel immediately or return if no data is available
    ///     
    /// Returns the next message available from the channel, if there is one, None otherwise.
    ///
    /// # Errors
    /// Various [errors](enum.ReadError.html) may occur such: a `writer` timeout is detected, end of channel is reached, channel is closed or channel data is corrupted.
    /// Once an error occurs, *any future read operation will fail*, so no more other records could ever be read from this channel.
    ///
    /// # Examples
    ///
    /// ```
    /// # use kekbit_core::tick::TickUnit::Nanos;
    /// # use kekbit_core::header::Header;
    /// # use kekbit_codecs::codecs::raw::RawBinDataFormat;
    /// use kekbit_core::shm::*;
    /// use crate::kekbit_core::api::Reader;
    /// # const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// # let writer = shm_writer(&test_tmp_dir.path(), &header, RawBinDataFormat).unwrap();
    /// let mut reader = shm_reader(&test_tmp_dir.path(), channel_id).unwrap();
    /// match reader.try_read() {
    ///        Ok(Some(buf)) =>println!("Read {}", std::str::from_utf8(buf).unwrap()),
    ///        Ok(None) => println!("Nothing to read"),
    ///        Err(err) =>println!("Read failed"),
    ///    }
    ///
    /// ```
    ///
    #[allow(clippy::cast_ptr_alignment)]
    fn try_read<'a>(&mut self) -> Result<Option<&'a [u8]>, ReadError> {
        let bytes_at_start = self.read_index;
        loop {
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
                self.expiration = END_OF_TIME;
                self.read_index += rec_size;
                if rec_len > 0 {
                    //otherwise is a heartbeat
                    return unsafe {
                        Ok(Some(std::slice::from_raw_parts(
                            self.data_ptr.add(crt_index + REC_HEADER_LEN as usize),
                            rec_len as usize,
                        )))
                    };
                }
            } else {
                match rec_len {
                    WATERMARK => {
                        if self.expiration == END_OF_TIME {
                            //start the timeout clock
                            self.expiration = self.header.tick_unit().nix_time() + self.header.timeout();
                            return Ok(None);
                        } else if self.expiration >= self.header.tick_unit().nix_time() {
                            return Ok(None);
                        } else {
                            warn!("Writer timeout detected. Channel will be abandoned. No more reads will be performed");
                            return Err(ReadError::Timeout {
                                timeout: self.expiration,
                            });
                        }
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
    }
}

///A non-blocking iterator over messages in the channel.
///Each call to next returns a message if there is one ready to be received.
///The iterator never blocks waiting for a message.
pub struct TryIter<'a> {
    inner: &'a mut ShmReader,
    available: bool,
}

impl<'a> Iterator for TryIter<'a> {
    type Item = &'a [u8];
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.available {
            match self.inner.try_read() {
                Ok(None) => None,
                Ok(record) => record,
                Err(_) => {
                    self.available = false;
                    None
                }
            }
        } else {
            None
        }
    }
    ///Returns (0,None) if records may be still available in the channel or (0,Some(0)) if
    ///no records will ever be available from this channel, such it can be use to know
    /// when to stop calling next on this iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.available {
            (0, None)
        } else {
            (0, Some(0))
        }
    }
}

// impl<'a> IntoIterator for &'a mut ShmReader {
//     type Item = IterResult<&'a [u8]>;
//     type IntoIter = Iter<'a>;
//     fn into_iter(self) -> Self::IntoIter {
//         Iter {
//             inner: self,
//             available: true,
//         }
//     }
// }
