//!Handles metadata associated with a channel.
use crate::api::ChannelError;
use crate::api::ChannelError::{IncompatibleVersion, InvalidCapacity, InvalidMaxMessageLength, InvalidSignature};
use crate::tick::TickUnit;
use crate::utils::{align, is_aligned, REC_HEADER_LEN};
use crate::version::Version;
use std::cmp::max;
use std::cmp::min;

const MIN_CAPACITY: u32 = 1024 * 16;
const HEADER_LEN: usize = 128;
const SIGNATURE: u64 = 0x2A54_4942_4B45_4B2A; //"*KEKBIT*" as bytes as u64

#[inline]
const fn compute_max_msg_len(capacity: u32) -> u32 {
    //if you reduce MIN_CAPACITY this may underflow!
    (capacity >> 7) - (REC_HEADER_LEN as u32)
}

/// Defines and validates the metadata associated with a channel.
#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    writer_id: u64,
    channel_id: u64,
    capacity: u32,
    max_msg_len: u32,
    timeout: u64,
    creation_time: u64,
    tick_unit: TickUnit,
    version: Version,
}

#[allow(clippy::len_without_is_empty)]
impl Header {
    /// Defines a new channel header.
    ///
    /// Return a struct that contains all the metadata required to be associated with a new channel.
    ///
    /// # Arguments
    ///
    /// * `writer_id` - Channel's writer identifier
    /// * `channel_id` - Channel's identifier
    /// * `capacity_hint` - Hint for the size of the channel - the maximum amount of data that can be wrote into the channel.
    ///                  Usually a successfully created channel will have a size very close to this hint, probably a little larger.
    /// * `max_msg_len_hint` - Hint for the maximum size of a message wrote into the channel. This cannot be larger than a certain fraction.
    ///        of the channel's capacity(1/128th), so the new created channel may have max message length value smaller than this hint.
    /// * `timeout` - Specifies the write inactivity time interval after each the reader will consider the channel abandoned by the writer.
    /// * `tick_unit` - Time unit used by the timeout and creation time attributes.        
    ///
    /// # Example
    ///
    /// ```
    /// use kekbit_core::tick::TickUnit::Nanos;
    /// use kekbit_core::header::*;
    ///     
    /// let producer_id: u64 = 111;
    /// let channel_id: u64 = 101;
    /// let capacity: u32 = 10_001;
    /// let max_msg_len: u32 = 100;
    /// let timeout: u64 = 10_000;
    /// let tick_unit = Nanos;
    /// let header = Header::new(channel_id, producer_id, capacity, max_msg_len, timeout, tick_unit);
    /// println!("{:?}", &header);
    /// ````
    ///
    ///
    #[inline]
    pub fn new(
        writer_id: u64,
        channel_id: u64,
        capacity_hint: u32,
        max_msg_len_hint: u32,
        timeout: u64,
        tick_unit: TickUnit,
    ) -> Header {
        let capacity = max(MIN_CAPACITY, align(capacity_hint));
        let max_msg_len = align(min(max_msg_len_hint + REC_HEADER_LEN, compute_max_msg_len(capacity)) as u32);
        let creation_time = tick_unit.nix_time();
        Header {
            writer_id,
            channel_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
            version: Version::latest(),
        }
    }
    ///Reads and `validates` the metadata from an existing memory mapped channel.
    ///
    ///Returns the metadata associated with the channel.
    ///
    /// # Arguments
    ///
    /// * `header` - Reference to a  byte array which should contain metadata associated with a given channel.
    ///              Usually points at the beginning of a memory mapped file used as storage for a kekbit channel.
    ///
    /// # Errors
    ///     
    /// An error will occur if data is corrupted or points to an incompatible version of kekbit channel.
    ///
    /// # Example
    ///
    ///```
    /// use memmap::MmapOptions;
    /// use std::fs::OpenOptions;
    ///
    /// # use kekbit_core::tick::TickUnit::Nanos;
    /// # use kekbit_core::header::Header;
    /// # use kekbit_codecs::codecs::raw::RawBinDataFormat;
    /// use kekbit_core::shm::*;
    /// # const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 4242;
    /// # let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let dir_path = test_tmp_dir.path();
    ///  # let writer = shm_writer(&test_tmp_dir.path(), &header, RawBinDataFormat).unwrap();
    ///
    /// let kek_file_name = storage_path(dir_path, channel_id);
    /// let kek_file = OpenOptions::new()
    ///  .write(true)
    ///  .read(true)
    ///  .open(&kek_file_name).unwrap();
    ///  let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.unwrap();
    ///  let buf = &mut mmap[..];
    ///  let header = Header::read(buf).unwrap();
    ///  println!("{:?}", &header);
    ///  ```
    ///    
    pub fn read(header: &[u8]) -> Result<Header, ChannelError> {
        assert!(header.len() >= HEADER_LEN);
        let mut offset = 0;
        let signature = Header::read_u64(header, offset);
        if signature != SIGNATURE {
            return Err(InvalidSignature {
                expected: SIGNATURE,
                actual: signature,
            });
        }
        offset += 8;
        let version: Version = Header::read_u64(header, 8).into();
        let latest = Version::latest();
        if !latest.is_compatible(version) {
            return Err(IncompatibleVersion {
                expected: latest.into(),
                actual: version.into(),
            });
        }
        offset += 8;
        let writer_id = Header::read_u64(header, offset);
        offset += 8;
        let channel_id = Header::read_u64(header, offset);
        offset += 8;
        let capacity = Header::read_u32(header, offset);
        if capacity < MIN_CAPACITY {
            return Err(InvalidCapacity {
                capacity,
                msg: "Capacity below minimum allowed of 10KB",
            });
        }
        if !is_aligned(MIN_CAPACITY) {
            return Err(InvalidCapacity {
                capacity,
                msg: "Capacity is not 8 bytes aligned",
            });
        }
        offset += 4;
        let max_msg_len = Header::read_u32(header, offset);
        if max_msg_len > align(compute_max_msg_len(capacity)) {
            return Err(InvalidMaxMessageLength {
                msg_len: max_msg_len,
                msg: "Max message lenght is too large",
            });
        }
        if !is_aligned(max_msg_len) {
            return Err(InvalidMaxMessageLength {
                msg_len: max_msg_len,
                msg: "Max message length is not 8 bytes aligned",
            });
        }
        offset += 4;
        let timeout = Header::read_u64(header, offset);
        offset += 8;
        let creation_time = Header::read_u64(header, offset);
        offset += 8;
        let tick_unit = TickUnit::from_id(header[offset]);
        //offset += 1;
        Ok(Header {
            version,
            writer_id,
            channel_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
        })
    }
    ///Writes kekbit metadata to a memory mapepd file.
    ///
    /// Returns  the lenght of the metadata
    ///
    /// # Arguments
    ///
    /// * `header` - Reference to a byte slice where metadata must be written.
    ///              Usually points at the beginning of a memory mapped file used as storage for a kekbit channel.
    ///
    /// # Example
    ///
    ///```
    /// use memmap::MmapOptions;
    /// use std::fs::OpenOptions;
    ///
    /// use kekbit_core::tick::TickUnit::Nanos;
    /// use kekbit_core::header::Header;
    /// use kekbit_core::shm::*;
    /// use std::fs::DirBuilder;
    ///
    /// const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 42;
    /// let test_tmp_dir = tempdir::TempDir::new("keksample").unwrap();
    /// let dir_path = test_tmp_dir.path().join(writer_id.to_string());
    /// let mut builder = DirBuilder::new();
    /// builder.recursive(true);
    /// builder.create(&dir_path).or_else(|err| Err(err.to_string())).unwrap();
    ///
    /// let kek_file_name = dir_path.join(format!("{}.kekbit", channel_id));
    /// let kek_file = OpenOptions::new()
    /// .write(true)
    /// .read(true)
    /// .create(true)
    /// .open(&kek_file_name)
    /// .or_else(|err| Err(err.to_string())).unwrap();
    ///
    /// let header = Header::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let total_len = (header.capacity() + header.len() as u32) as u64;
    /// kek_file.set_len(total_len).or_else(|err| Err(err.to_string())).unwrap();
    /// let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.unwrap();
    /// let buf = &mut mmap[..];
    /// header.write_to(buf);
    /// mmap.flush().unwrap();
    /// ```
    #[inline]
    pub fn write_to(&self, header: &mut [u8]) -> usize {
        assert!(self.len() <= header.len());
        header[0..8].clone_from_slice(&SIGNATURE.to_le_bytes());
        let latest_v: u64 = Version::latest().into();
        header[8..16].clone_from_slice(&latest_v.to_le_bytes());
        header[16..24].clone_from_slice(&self.writer_id.to_le_bytes());
        header[24..32].clone_from_slice(&self.channel_id.to_le_bytes());
        header[32..36].clone_from_slice(&self.capacity.to_le_bytes());
        header[36..40].clone_from_slice(&self.max_msg_len.to_le_bytes());
        header[40..48].clone_from_slice(&self.timeout.to_le_bytes());
        header[48..56].clone_from_slice(&self.creation_time.to_le_bytes());
        header[56] = self.tick_unit.id();
        let last = 57;
        for item in header.iter_mut().take(HEADER_LEN).skip(last) {
            *item = 0u8;
        }
        self.len()
    }

    #[inline]
    fn read_u64(header: &[u8], offset: usize) -> u64 {
        assert!(offset + 8 < HEADER_LEN);
        u64::from_le_bytes([
            header[offset],
            header[offset + 1],
            header[offset + 2],
            header[offset + 3],
            header[offset + 4],
            header[offset + 5],
            header[offset + 6],
            header[offset + 7],
        ])
    }

    #[inline]
    fn read_u32(header: &[u8], offset: usize) -> u32 {
        assert!(offset + 4 < HEADER_LEN);
        u32::from_le_bytes([header[offset], header[offset + 1], header[offset + 2], header[offset + 3]])
    }

    ///Returns the metadata version
    #[inline]
    pub fn version(&self) -> String {
        self.version.to_string()
    }

    ///Returns the channel identifier
    #[inline]
    pub fn channel_id(&self) -> u64 {
        self.channel_id
    }

    ///Returns the channel writer identifier
    #[inline]
    pub fn writer_id(&self) -> u64 {
        self.writer_id
    }

    ///Returns the capacity of the channel
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    ///Returns the maximum message size allowed
    #[inline]
    pub fn max_msg_len(&self) -> u32 {
        self.max_msg_len
    }
    ///Returns the inactivity time interval after each the reader will consider the channel abandoned by the writer.
    #[inline]
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    ///Returns the channel creation time
    #[inline]
    pub fn creation_time(&self) -> u64 {
        self.creation_time
    }
    ///Returns the time unit used by the channel creation time and the timeout attributes.
    #[inline]
    pub fn tick_unit(&self) -> TickUnit {
        self.tick_unit
    }
    #[inline]
    ///Returns  the length of the metadata. For any given version the length is the same.
    ///In the current version it is 128 bytes.
    pub const fn len(&self) -> usize {
        HEADER_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_read_write_header() {
        let producer_id: u64 = 111;
        let channel_id: u64 = 101;
        let capacity: u32 = 10_001;
        let max_msg_len: u32 = 100;
        let timeout: u64 = 10_000;
        let tick_unit = TickUnit::Nanos;
        let head = Header::new(producer_id, channel_id, capacity, max_msg_len, timeout, tick_unit);
        let mut data = vec![0u8; HEADER_LEN];
        assert!(head.write_to(&mut data) == HEADER_LEN);
        assert!(Header::read(&data).unwrap() == head);
        assert_eq!(head.tick_unit(), TickUnit::Nanos);
        assert_eq!(head.timeout(), timeout);
        assert_eq!(head.version(), Version::latest().to_string());
        assert!(head.creation_time() < tick_unit.nix_time());
        assert_eq!(head.len(), 128);
        assert_eq!(head.writer_id(), producer_id);
    }
}
