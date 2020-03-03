use super::utils::{align, load_atomic_u64, CLOSE, REC_HEADER_LEN, U64_SIZE, WATERMARK};
use super::Metadata;
use crate::api::ReadError::*;
use crate::api::{ChannelError, ReadError, Reader};
use log::{error, info, warn};
use memmap::MmapMut;
use std::cmp::Ordering::*;
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
#[derive(Debug)]
pub struct ShmReader {
    metadata: Metadata,
    data_ptr: *const u8,
    read_index: u32,
    expiration: u64,
    failure: Option<ReadError>,
    _mmap: MmapMut,
}

impl ShmReader {
    #[allow(clippy::cast_ptr_alignment)]
    pub(super) fn new(mut mmap: MmapMut) -> Result<ShmReader, ChannelError> {
        let buf = &mut mmap[..];
        let metadata = Metadata::read(buf)?;
        let metadata_ptr = buf.as_ptr() as *mut u64;
        let data_ptr = unsafe { metadata_ptr.add(metadata.len() as usize) } as *const u8;
        info!("Kekbit Reader successfully created");
        Ok(ShmReader {
            metadata,
            data_ptr,
            read_index: 0,
            expiration: END_OF_TIME,
            failure: None,
            _mmap: mmap,
        })
    }
    ///Returns a reference to the [Metadata](struct.Metadata.html) associated with this channel
    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    ///Returns the current read position. It is also the `total` amount of bytes read
    ///so far(including bytes from record headers and the one used for record padding)
    pub fn position(&self) -> u32 {
        self.read_index
    }

    /// Returns A *non-blocking* iterator over messages in the channel.
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
    ///
    #[inline]
    pub fn try_iter(&mut self) -> TryIter {
        TryIter { inner: self }
    }

    #[inline]
    fn record_failure(&mut self, failure: ReadError) -> ReadError {
        if self.failure.is_none() {
            self.failure = Some(failure);
        }
        failure
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
    /// # use kekbit::core::TickUnit::Nanos;
    /// use kekbit::core::*;
    /// use kekbit::api::*;
    /// use crate::kekbit::api::Reader;
    /// # const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// # let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// # let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
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
        loop {
            //will simply skip all the hearbeats. We may add  a limit of the heartbeats we skip in the future if
            //they proof to be too many.
            let crt_index = self.read_index as usize;
            debug_assert!(crt_index + U64_SIZE < self.metadata.capacity() as usize);
            let rec_len: u64 = unsafe { load_atomic_u64(self.data_ptr.add(crt_index) as *mut u64, Ordering::Acquire) };
            if rec_len <= self.metadata.max_msg_len() as u64 {
                let rec_size = align(REC_HEADER_LEN + rec_len as u32);
                debug_assert!((crt_index + rec_size as usize) < self.metadata.capacity() as usize);
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
                return match rec_len {
                    WATERMARK => {
                        let tick = self.metadata.tick_unit().nix_time();
                        match self.expiration.cmp(&tick) {
                            Less | Equal => {
                                warn!("Writer timeout detected. Channel will be abandoned. No more reads will be performed");
                                Err(self.record_failure(Timeout(self.expiration)))
                            }
                            Greater => {
                                if self.expiration == END_OF_TIME {
                                    self.expiration = tick + self.metadata.timeout();
                                }
                                Ok(None)
                            }
                        }
                    }
                    CLOSE => {
                        info!("Producer closed channel");
                        Err(self.record_failure(Closed))
                    }
                    _ => {
                        error!(
                            "Channel corrupted. Unknown Marker {:#016X} at position {} ",
                            rec_len, self.read_index,
                        );
                        Err(self.record_failure(Failed))
                    }
                };
            }
        }
    }

    ///Check if the channel is exhausted and what was the reason of exhaustion.
    /// Could be also use to check if an iterator will ever yield a record again.
    #[inline]
    fn exhausted(&self) -> Option<ReadError> {
        self.failure
    }
}

///A non-blocking iterator over messages in the channel.
///Each call to `next` returns a message if there is one ready to be received.
///The iterator never blocks waiting for a message.
pub struct TryIter<'a> {
    inner: &'a mut ShmReader,
}

impl<'a> Iterator for TryIter<'a> {
    type Item = &'a [u8];
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.exhausted().is_none() {
            match self.inner.try_read() {
                Ok(None) => None,
                Ok(record) => record,
                Err(_) => None,
            }
        } else {
            None
        }
    }
    ///Returns (0, None) if records may be still available in the channel or (0, Some(0)) if
    ///the channel is exhausted. Use this method if you want to know if future `next` calls will ever produce more items.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.inner.exhausted().is_none() {
            (0, None)
        } else {
            (0, Some(0))
        }
    }
}
