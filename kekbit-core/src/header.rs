use crate::tick::TickUnit;
use crate::utils::{align, is_aligned, REC_HEADER_LEN};
use crate::version::{Version, V_0_0_1};
use std::cmp::max;
use std::cmp::min;

const MIN_CAPACITY: u32 = 1024;
const HEADER_LEN: usize = 128;
const MAGIC_U64: u64 = 0x2A54_4942_4B45_4B2A; //"*KEKBIT*" as bytes as u64
const LATEST: Version = V_0_0_1;

#[inline]
const fn compute_max_msg_len(capacity: u32) -> u32 {
    capacity >> 7
}

#[inline]
pub fn latest_version() -> String {
    LATEST.to_string()
}

#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    producer_id: u64,
    channel_id: u64,
    capacity: u32,
    max_msg_len: u32,
    timeout: u64,
    creation_time: u64,
    tick_unit: TickUnit,
    version: Version,
}

impl Header {
    pub fn new(
        producer_id: u64,
        channel_id: u64,
        suggested_capacity: u32,
        suggested_max_msg_len: u32,
        timeout: u64,
        creation_time: u64,
        tick_unit: TickUnit,
    ) -> Header {
        let capacity = max(MIN_CAPACITY, align(suggested_capacity));
        let max_msg_len = align(min(suggested_max_msg_len, compute_max_msg_len(capacity)) + REC_HEADER_LEN as u32);
        Header {
            version: LATEST,
            producer_id,
            channel_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
        }
    }
    pub fn read(header: &[u8]) -> Result<Header, String> {
        assert!(header.len() >= HEADER_LEN);
        let mut offset = 0;
        let magic = Header::read_u64(header, offset);
        if magic != MAGIC_U64 {
            return Err(format!("Invalid magic header {:X}. Expected {:X}", magic, MAGIC_U64));
        }
        offset += 8;
        let version: Version = Header::read_u64(header, 8).into();
        if !LATEST.is_compatible(version) {
            return Err(format!(
                "Invalid file version {}. Expected something compatible with {}",
                version, LATEST
            ));
        }
        offset += 8;
        let producer_id = Header::read_u64(header, offset);
        offset += 8;
        let channel_id = Header::read_u64(header, offset);
        offset += 8;
        let capacity = Header::read_u32(header, offset);
        if capacity < MIN_CAPACITY || !is_aligned(MIN_CAPACITY) {
            return Err(format!(
                "Invalid store capacity {}. Expected something align and not smaller than {}",
                capacity, MIN_CAPACITY
            ));
        }
        offset += 4;
        let max_msg_len = Header::read_u32(header, offset);
        let expected_msg_len = align(min(max_msg_len, compute_max_msg_len(capacity)) + REC_HEADER_LEN as u32);
        if max_msg_len != expected_msg_len {
            return Err(format!(
                "Invalid max message length {}. Expected {}",
                max_msg_len, expected_msg_len
            ));
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
            producer_id,
            channel_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
        })
    }

    pub fn write(&self, header: &mut [u8]) -> usize {
        assert!(HEADER_LEN <= header.len());
        header[0..8].clone_from_slice(&MAGIC_U64.to_le_bytes());
        let latest_v: u64 = LATEST.into();
        header[8..16].clone_from_slice(&latest_v.to_le_bytes());
        header[16..24].clone_from_slice(&self.producer_id.to_le_bytes());
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
        HEADER_LEN
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

    #[inline]
    pub fn version(&self) -> String {
        self.version.to_string()
    }

    #[inline]
    pub fn channel_id(&self) -> u64 {
        self.channel_id
    }

    #[inline]
    pub fn producer_id(&self) -> u64 {
        self.producer_id
    }

    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    #[inline]
    pub fn max_msg_len(&self) -> u32 {
        self.max_msg_len
    }
    #[inline]
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    #[inline]
    pub fn creation_time(&self) -> u64 {
        self.creation_time
    }

    #[inline]
    pub fn tick_unit(&self) -> TickUnit {
        self.tick_unit
    }
    #[inline]
    pub const fn len(&self) -> usize {
        HEADER_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_header() {
        let producer_id: u64 = 111;
        let channel_id: u64 = 101;
        let capacity: u32 = 10001;
        let max_msg_len: u32 = 100;
        let timeout: u64 = 10000;
        let tick_unit = TickUnit::Nanos;
        let creation_time: u64 = 1111111;
        let head = Header::new(
            channel_id,
            producer_id,
            capacity,
            max_msg_len,
            timeout,
            creation_time,
            tick_unit,
        );
        let mut data = vec![0u8; HEADER_LEN];
        assert!(head.write(&mut data) == HEADER_LEN);
        assert!(Header::read(&data).unwrap() == head);
    }
}
