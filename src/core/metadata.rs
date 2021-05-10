//!Provides access to metadata associated with a channel.
use super::utils::{align, is_aligned, REC_HEADER_LEN};
use super::version::Version;
use super::TickUnit;
use crate::api::ChannelError;
use crate::api::ChannelError::{IncompatibleVersion, InvalidCapacity, InvalidMaxMessageLength, InvalidSignature};
use std::cmp::max;
use std::cmp::min;

const MIN_CAPACITY: u32 = 1024 * 16;
const METADATA_LEN: usize = 128;
const SIGNATURE: u64 = 0x2A54_4942_4B45_4B2A; //"*KEKBIT*" as bytes as u64

#[inline]
const fn compute_max_msg_len(capacity: u32) -> u32 {
    //if you reduce MIN_CAPACITY this may underflow!
    (capacity >> 7) - (REC_HEADER_LEN as u32)
}

/// Defines and validates the metadata associated with a channel.
#[derive(PartialEq, Eq, Debug)]
pub struct Metadata {
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
impl Metadata {
    /// Defines a new channel metadata.
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
    /// use kekbit::core::TickUnit::Nanos;
    /// use kekbit::core::*;
    ///     
    /// let producer_id: u64 = 111;
    /// let channel_id: u64 = 101;
    /// let capacity: u32 = 10_001;
    /// let max_msg_len: u32 = 100;
    /// let timeout: u64 = 10_000;
    /// let tick_unit = Nanos;
    /// let metadata = Metadata::new(channel_id, producer_id, capacity, max_msg_len, timeout, tick_unit);
    /// println!("{:?}", &metadata);
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
    ) -> Metadata {
        let capacity = max(MIN_CAPACITY, align(capacity_hint));
        let max_msg_len = align(min(max_msg_len_hint + REC_HEADER_LEN, compute_max_msg_len(capacity)) as u32);
        let creation_time = tick_unit.nix_time();
        Metadata {
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
    /// * `metadata` - Reference to a  byte array which should contain metadata associated with a given channel.
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
    /// # use kekbit::core::TickUnit::Nanos;
    /// # use kekbit::core::Metadata;
    /// use kekbit::core::*;
    /// use kekbit::api::*;
    /// # const FOREVER: u64 = 99_999_999_999;
    /// let writer_id = 1850;
    /// let channel_id = 4242;
    /// # let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let test_tmp_dir = tempdir::TempDir::new("kektest").unwrap();
    /// let dir_path = test_tmp_dir.path();
    ///  # let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
    ///
    /// let kek_file_name = storage_path(dir_path, channel_id);
    /// let kek_file = OpenOptions::new()
    ///  .write(true)
    ///  .read(true)
    ///  .open(&kek_file_name).unwrap();
    ///  let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.unwrap();
    ///  let buf = &mut mmap[..];
    ///  let metadata = Metadata::read(buf).unwrap();
    ///  println!("{:?}", &metadata);
    ///  ```
    ///    
    pub fn read(metadata: &[u8]) -> Result<Metadata, ChannelError> {
        assert!(metadata.len() >= METADATA_LEN);
        let mut offset = 0;
        let signature = Metadata::read_u64(metadata, offset);
        if signature != SIGNATURE {
            return Err(InvalidSignature {
                expected: SIGNATURE,
                actual: signature,
            });
        }
        offset += 8;
        let version: Version = Metadata::read_u64(metadata, 8).into();
        let latest = Version::latest();
        if !latest.is_compatible(version) {
            return Err(IncompatibleVersion {
                expected: latest.into(),
                actual: version.into(),
            });
        }
        offset += 8;
        let writer_id = Metadata::read_u64(metadata, offset);
        offset += 8;
        let channel_id = Metadata::read_u64(metadata, offset);
        offset += 8;
        let capacity = Metadata::read_u32(metadata, offset);
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
        let max_msg_len = Metadata::read_u32(metadata, offset);
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
        let timeout = Metadata::read_u64(metadata, offset);
        offset += 8;
        let creation_time = Metadata::read_u64(metadata, offset);
        offset += 8;
        let tick_unit = TickUnit::from_id(metadata[offset]);
        //offset += 1;
        Ok(Metadata {
            writer_id,
            channel_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
            version,
        })
    }
    ///Writes kekbit metadata to a memory mapepd file.
    ///
    /// Returns  the lenght of the metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - Reference to a byte slice where metadata must be written.
    ///              Usually points at the beginning of a memory mapped file used as storage for a kekbit channel.
    ///
    /// # Example
    ///
    ///```
    /// use memmap::MmapOptions;
    /// use std::fs::OpenOptions;
    ///
    /// use kekbit::core::TickUnit::Nanos;
    /// use kekbit::core::Metadata;
    /// use kekbit::core::*;
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
    /// let metadata = Metadata::new(writer_id, channel_id, 300_000, 1000, FOREVER, Nanos);
    /// let total_len = (metadata.capacity() + metadata.len() as u32) as u64;
    /// kek_file.set_len(total_len).or_else(|err| Err(err.to_string())).unwrap();
    /// let mut mmap = unsafe { MmapOptions::new().map_mut(&kek_file) }.unwrap();
    /// let buf = &mut mmap[..];
    /// metadata.write_to(buf);
    /// mmap.flush().unwrap();
    /// ```
    #[inline]
    pub fn write_to(&self, metadata: &mut [u8]) -> usize {
        assert!(self.len() <= metadata.len());
        metadata[0..8].clone_from_slice(&SIGNATURE.to_le_bytes());
        let latest_v: u64 = Version::latest().into();
        metadata[8..16].clone_from_slice(&latest_v.to_le_bytes());
        metadata[16..24].clone_from_slice(&self.writer_id.to_le_bytes());
        metadata[24..32].clone_from_slice(&self.channel_id.to_le_bytes());
        metadata[32..36].clone_from_slice(&self.capacity.to_le_bytes());
        metadata[36..40].clone_from_slice(&self.max_msg_len.to_le_bytes());
        metadata[40..48].clone_from_slice(&self.timeout.to_le_bytes());
        metadata[48..56].clone_from_slice(&self.creation_time.to_le_bytes());
        metadata[56] = self.tick_unit.id();
        let last = 57;
        for item in metadata.iter_mut().take(METADATA_LEN).skip(last) {
            *item = 0u8;
        }
        self.len()
    }

    #[inline]
    fn read_u64(metadata: &[u8], offset: usize) -> u64 {
        assert!(offset + 8 < METADATA_LEN);
        u64::from_le_bytes([
            metadata[offset],
            metadata[offset + 1],
            metadata[offset + 2],
            metadata[offset + 3],
            metadata[offset + 4],
            metadata[offset + 5],
            metadata[offset + 6],
            metadata[offset + 7],
        ])
    }

    #[inline]
    fn read_u32(metadata: &[u8], offset: usize) -> u32 {
        assert!(offset + 4 < METADATA_LEN);
        u32::from_le_bytes([
            metadata[offset],
            metadata[offset + 1],
            metadata[offset + 2],
            metadata[offset + 3],
        ])
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
        METADATA_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_read_write_metadata() {
        let producer_id: u64 = 111;
        let channel_id: u64 = 101;
        let capacity: u32 = 10_001;
        let max_msg_len: u32 = 100;
        let timeout: u64 = 10_000;
        let tick_unit = TickUnit::Nanos;
        let head = Metadata::new(producer_id, channel_id, capacity, max_msg_len, timeout, tick_unit);
        let mut data = vec![0u8; METADATA_LEN];
        assert!(head.write_to(&mut data) == METADATA_LEN);
        assert!(Metadata::read(&data).unwrap() == head);
        assert_eq!(head.tick_unit(), TickUnit::Nanos);
        assert_eq!(head.timeout(), timeout);
        assert_eq!(head.version(), Version::latest().to_string());
        assert!(head.creation_time() < tick_unit.nix_time());
        assert_eq!(head.len(), 128);
        assert_eq!(head.writer_id(), producer_id);
    }
}
