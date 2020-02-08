use crate::api::{WriteError, Writer};
use crate::header::Header;
use crate::utils::{align, store_atomic_u64, CLOSE, REC_HEADER_LEN, WATERMARK};
use log::{debug, error, info};
use memmap::MmapMut;
use std::ptr::copy_nonoverlapping;
use std::result::Result;
use std::sync::atomic::Ordering;
/// An implementation of the [Writer](trait.Writer.html) which access a persistent channel through
/// memory mapping. A `ShmWriter` must be created using the [shm_writer](fn.shm_writer.html) function.
/// Any `ShmWriter` exclusively holds the channel is bound to, and it is *not thread safe*.
/// If multiple threads must write into a channel they should be externally synchronized.
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
    fn write_metadata(&mut self, write_ptr: *mut u64, len: u64, aligned_rec_len: u32) {
        unsafe {
            store_atomic_u64(write_ptr.add(aligned_rec_len as usize), WATERMARK, Ordering::Release);
        }
        store_atomic_u64(write_ptr, len, Ordering::Release);
    }
}

impl Writer for ShmWriter {
    /// Writes a  message into the channel. This operation will copy the message into the channel storage.
    /// While this is a non blocking operation, only one write should be executed at any given time.
    ///
    /// Returns the total amount of bytes wrote into the channel which includes, the size of the message,
    /// the size of the message header and the amount of padding add to that message.
    ///
    /// # Arguments
    ///
    /// *`data` - The buffer which contains the data which is going to be wrote into the channel.
    /// * `len` - The amount of data which is going to be wrote into to he channel
    ///
    /// # Errors
    ///
    /// Two types of [failures](enum.WriteError.html) may occur: message size is larger than the maximum allowed,
    /// or the there is not enough space in the channel to write that message. In the second case, a future write may succeed,
    /// if the message has a smaller size that the current one.
    ///
    ////// # Examples
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
    /// let capacity = 30_000;
    /// let max_msg_len = 100;
    /// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos.nix_time(), Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
    /// let msg = "There are 10 kinds of people: those who know binary and those who don't";
    /// let msg_data = msg.as_bytes();
    /// writer.write(&msg_data, msg_data.len() as u32).unwrap();
    /// ```
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
    /// use kekbit_core::tick::TickUnit::Nanos;
    /// use kekbit_core::shm::*;
    /// use kekbit_core::header::Header;
    /// use kekbit_core::api::Writer;
    ///
    /// const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// let capacity = 30_000;
    /// let max_msg_len = 100;
    /// let header = Header::new(writer_id, channel_id, capacity, max_msg_len, FOREVER, Nanos.nix_time(), Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let mut writer = shm_writer(&test_tmp_dir.path(), &header).unwrap();
    /// let msg = "There are 10 kinds of people: those who know binary and those who don't";
    /// let msg_data = msg.as_bytes();
    /// writer.write(&msg_data, msg_data.len() as u32).unwrap();
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
    ///Returns the amount of space still available into this channel.
    #[inline]
    pub fn available(&self) -> u32 {
        self.header.capacity() - self.write_offset
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
