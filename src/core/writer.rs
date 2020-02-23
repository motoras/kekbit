use super::utils::{align, store_atomic_u64, CLOSE, REC_HEADER_LEN, WATERMARK};
use super::Header;
use crate::api::ChannelError::AccessError;
use crate::api::{ChannelError, Encodable, WriteError, Writer};
use log::{debug, error, info};
use memmap::MmapMut;
use std::cmp::min;
use std::io::Error;
use std::io::ErrorKind::WriteZero;
use std::io::Write;
use std::ptr::copy_nonoverlapping;
use std::result::Result;
use std::sync::atomic::Ordering;

/// Implementation of the [Writer](trait.Writer.html) which access a persistent channel through
/// memory mapping,  A `ShmWriter` must be created using the [shm_writer](fn.shm_writer.html) function.
/// Any `ShmWriter` exclusively holds the channel is bound to, and it is *not thread safe*.
/// If multiple threads must write into a channel they should be externally synchronized.
///
/// # Examples
///
/// ```
/// use kekbit::core::TickUnit::Nanos;
/// use kekbit::core::*;
/// use kekbit::core::Header;
/// use kekbit::api::Writer;
///
/// const FOREVER: u64 = 99_999_999_999;
/// let writer_id = 1850;
/// let channel_id = 42;
/// let capacity = 3000;
/// let max_msg_len = 100;
/// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos);
/// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
/// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
/// writer.heartbeat().unwrap();
/// ```
pub struct ShmWriter {
    header: Header,
    data_ptr: *mut u8,
    write_offset: u32,
    mmap: MmapMut,
    write: KekWrite,
}

impl ShmWriter {
    #[allow(clippy::cast_ptr_alignment)]
    pub(super) fn new(mut mmap: MmapMut) -> Result<ShmWriter, ChannelError> {
        let buf = &mut mmap[..];
        let header = Header::read(buf)?;
        let header_ptr = buf.as_ptr() as *mut u64;
        let head_len = header.len();
        let data_ptr = unsafe { header_ptr.add(head_len) } as *mut u8;
        let write = KekWrite::new(data_ptr, header.max_msg_len() as usize);
        let mut writer = ShmWriter {
            header,
            data_ptr,
            write_offset: 0,
            mmap,
            write,
        };
        info!(
            "Kekbit channel writer created. Size is {}MB. Max msg size {}KB",
            writer.header.capacity() / 1_000_000,
            writer.header.max_msg_len() / 1_000
        );
        //sent the very first original heart bear
        match writer.heartbeat() {
            Ok(_) => {
                info!("Initial hearbeat successfully sent!");
                Ok(writer)
            }
            Err(we) => Err(AccessError {
                reason: format!("Initial heartbeat failed!. Reason {:?}", we),
            }),
        }
    }

    #[inline(always)]
    fn write_metadata(&mut self, write_ptr: *mut u64, len: u64, aligned_rec_len: u32) {
        unsafe {
            //we should always have the 8 bytes required by WATERMARK as they are acounted in the Footer
            store_atomic_u64(write_ptr.add(aligned_rec_len as usize), WATERMARK, Ordering::Release);
        }
        store_atomic_u64(write_ptr, len, Ordering::Release);
    }
}

impl Writer for ShmWriter {
    /// Writes a message into the channel. This operation will encode the data directly into  channel.
    /// While this is a non blocking operation, only one write should be executed at any given time.
    ///
    /// Returns the total amount of bytes wrote into the channel which includes, the size of the message,
    /// the size of the message header and the amount of padding add to that message.
    ///
    /// # Arguments
    ///
    /// * `data` - The  data which to encode and  write into the channel.
    ///
    /// # Errors
    ///
    /// Two kinds of [failures](enum.WriteError.html) may occur. One if the encoding operation failed, the other if the channel
    /// rejected the message for reasons such data is too large or no space is available in the channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use kekbit::core::TickUnit::Nanos;
    /// use kekbit::core::*;
    /// use kekbit::core::Header;
    /// use kekbit::api::Writer;
    ///
    /// const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// let capacity = 30_000;
    /// let max_msg_len = 100;
    /// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
    /// let msg = "There are 10 kinds of people: those who know binary and those who don't";
    /// let msg_data = msg.as_bytes();
    /// writer.write(&msg_data).unwrap();
    /// ```
    ///
    #[allow(clippy::cast_ptr_alignment)]
    fn write(&mut self, data: &impl Encodable) -> Result<u32, WriteError> {
        let read_head_ptr = unsafe { self.data_ptr.add(self.write_offset as usize) };
        let write_ptr = unsafe { read_head_ptr.add(REC_HEADER_LEN as usize) };
        let available = self.available();
        if available <= REC_HEADER_LEN {
            return Err(WriteError::ChannelFull);
        }
        let len = min(self.header.max_msg_len(), available - REC_HEADER_LEN) as usize;
        let write_res = data.encode(self.write.reset(write_ptr, len));
        match write_res {
            Ok(_) => {
                if !self.write.failed {
                    let aligned_rec_len = align(self.write.total as u32 + REC_HEADER_LEN);
                    self.write_metadata(read_head_ptr as *mut u64, self.write.total as u64, aligned_rec_len >> 3);
                    self.write_offset += aligned_rec_len;
                    Ok(aligned_rec_len)
                } else {
                    Err(WriteError::NoSpaceForRecord)
                }
            }
            Err(io_err) => Err(WriteError::EncodingError(io_err)),
        }
    }
    ///Push a heartbeat message into the channel. Hearbeats are zero sized messages which do not need encoding.
    ///Reader should never activate callbacks for heartbeat messsages.
    ///
    /// Returns RecordHeaderLen, 8 in the current version if the operation succeeds.
    ///
    /// # Errors
    ///
    /// If the operation fails a *ChannelFull* error will be returned, which signals that the channel will not accept any new messages.
    ///
    #[allow(clippy::cast_ptr_alignment)]
    #[inline]
    fn heartbeat(&mut self) -> Result<u32, WriteError> {
        let read_head_ptr = unsafe { self.data_ptr.add(self.write_offset as usize) };
        let available = self.available();
        if available <= REC_HEADER_LEN {
            return Err(WriteError::ChannelFull);
        }
        let aligned_rec_len = REC_HEADER_LEN; //no need to align REC_HEADER)LEN must be align
        self.write_metadata(read_head_ptr as *mut u64, 0u64, aligned_rec_len >> 3);
        self.write_offset += aligned_rec_len;
        Ok(aligned_rec_len)
    }

    /// Flushes the channel's outstanding memory map modifications to disk. Calling  this method explicitly
    /// it is not encouraged as flushing does occur automatically and comes with a performance penalty.
    /// It should be used only if for various reasons a writer wants to persist the channel data to the disk
    /// at a higher rate than is done automatically.
    ///
    /// Returns Ok(()) if the operation succeeds.
    ///
    /// # Errors
    ///
    /// If flushing fails an I/O error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use kekbit::core::TickUnit::Nanos;
    /// use kekbit::core::*;
    /// use kekbit::core::Header;
    /// use kekbit::api::Writer;
    ///
    /// const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// let capacity = 30_000;
    /// let max_msg_len = 100;
    /// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
    /// let msg = "There are 10 kinds of people: those who know binary and those who don't";
    /// let msg_data = msg.as_bytes();
    /// writer.write(&msg_data).unwrap();
    /// writer.flush().unwrap();
    /// ```
    #[inline]
    fn flush(&mut self) -> Result<(), std::io::Error> {
        debug!("Flushing the channel");
        self.mmap.flush()
    }
}
impl Drop for ShmWriter {
    /// Marks this channel as `closed`, flushes the changes to the disk, and removes the memory mapping.
    fn drop(&mut self) {
        let write_index = self.write_offset;
        info!("Closing message queue..");
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            //we should always have the 8 bytes required by CLOSE as they are acounted in the Footer
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
    ///Returns the amount of space in this channel still available for write.
    #[inline]
    pub fn available(&self) -> u32 {
        (self.header.capacity() - self.write_offset) & 0xFFFF_FFF8 //rounded down to alignement
    }
    ///Returns the amount of data written into this channel.
    #[inline]
    pub fn write_offset(&self) -> u32 {
        self.write_offset
    }

    ///Returns a reference to the [Header](struct.Header.html) associated with this channel.
    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }
}

struct KekWrite {
    write_ptr: *mut u8,
    max_size: usize,
    total: usize,
    failed: bool,
}

impl KekWrite {
    #[inline]
    fn new(write_ptr: *mut u8, max_size: usize) -> Self {
        KekWrite {
            write_ptr,
            max_size,
            total: 0,
            failed: false,
        }
    }
    #[inline]
    fn reset(&mut self, write_ptr: *mut u8, max_size: usize) -> &mut Self {
        self.write_ptr = write_ptr;
        self.max_size = max_size;
        self.total = 0;
        self.failed = false;
        self
    }
}

impl Write for KekWrite {
    #[inline]
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        if self.failed {
            return Ok(0);
        }
        let data_len = data.len();
        if self.total + data_len > self.max_size {
            self.failed = true;
            return Err(Error::new(
                WriteZero,
                format!(
                    "Data larger than maximum allowed {} > {}",
                    self.total + data_len,
                    self.max_size
                ),
            ));
        }
        unsafe {
            let crt_ptr = self.write_ptr.add(self.total as usize);
            copy_nonoverlapping(data.as_ptr(), crt_ptr, data_len);
        }
        self.total += data_len;
        Ok(data_len)
    }
    #[inline]
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_write() {
        let mut raw_data: [u8; 1000] = [0; 1000];
        let write_ptr = raw_data.as_mut_ptr();
        let mut kw = KekWrite::new(write_ptr, 20);
        kw.flush().unwrap(); //should never crash as it does nothing
        let d1: [u8; 10] = [1; 10];
        let r1 = kw.write(&d1).unwrap();
        assert_eq!(kw.total, r1);
        assert!(!kw.failed);
        for i in 0..10 {
            assert_eq!(raw_data[i], 1);
        }
        kw.flush().unwrap(); //should never crash as it does nothing
        let r2 = kw.write(&d1).unwrap();
        assert_eq!(kw.total, r1 + r2);
        assert!(!kw.failed);
        for i in 10..20 {
            assert_eq!(raw_data[i], 1);
        }
        let r3 = kw.write(&d1);
        assert_eq!(r3.unwrap_err().kind(), std::io::ErrorKind::WriteZero);
        assert!(kw.failed);
        kw.reset(write_ptr, 15);
        assert!(!kw.failed);
        let d2: [u8; 10] = [2; 10];
        let r4 = kw.write(&d2).unwrap();
        assert_eq!(kw.total, r4);
        assert!(!kw.failed);
        for i in 0..10 {
            assert_eq!(raw_data[i], 2);
        }
        assert_eq!(kw.total, 10);
        let r5 = kw.write(&d2);
        assert_eq!(r5.unwrap_err().kind(), std::io::ErrorKind::WriteZero);
        assert!(kw.failed);
        assert_eq!(kw.total, 10);
        //once it fails it will never recover, even if it has enough space
        let r6 = kw.write(&d2[0..3]).unwrap();
        assert_eq!(0, r6);
        assert!(kw.failed);
        assert_eq!(kw.total, 10);
    }
}
