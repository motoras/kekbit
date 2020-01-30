use crate::tick::TickUnit;
use crate::version::{Version, V_0_0_1};
use std::fmt::Display;
use std::fmt::Formatter;

pub const HEADER_LEN: usize = 128;
pub const MAGIC_U64: u64 = 0x2A54_4942_4B45_4B2A; //"*KEKBIT*" as bytes as u64
pub const LATEST: Version = V_0_0_1;

#[derive(PartialEq, Debug)]
pub enum Status {
    Open(u64),
    Closed(u64),
    ForcedClosed(u64),
}

impl Status {
    #[inline]
    fn status_id(&self) -> u64 {
        match self {
            Status::Open(_) => 0,
            Status::Closed(_) => 255,
            Status::ForcedClosed(_) => 512,
        }
    }
    #[inline]
    fn from_id_and_ts(id: u64, timestamp: u64) -> Option<Status> {
        match id {
            0 => Some(Status::Open(timestamp)),
            255 => Some(Status::Closed(timestamp)),
            512 => Some(Status::ForcedClosed(timestamp)),
            _ => None,
        }
    }
    #[inline]
    fn timestamp(&self) -> u64 {
        match self {
            Status::Open(ts) => *ts,
            Status::Closed(ts) => *ts,
            Status::ForcedClosed(ts) => *ts,
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let res = match self {
            Status::Open(_) => "Open",
            Status::Closed(_) => "Closed",
            Status::ForcedClosed(_) => "ForcedClosed",
        };
        write!(f, "{}", res)
    }
}

#[inline]
pub fn check_header(header: &[u8]) -> Result<(), String> {
    let magic = magic(header);
    if magic != MAGIC_U64 {
        return Err(format!(
            "Invalid magic header {:X}. Expected {:X}",
            magic, MAGIC_U64
        ));
    }
    let v = version(header);
    if !LATEST.is_compatible(v) {
        return Err(format!(
            "Invalid file version {}. Expected something compatible with {}",
            v, LATEST
        ));
    }
    Ok(())
}

#[inline]
pub fn write_header(
    header: &mut [u8],
    producer_id: u64,
    channel_id: u64,
    capacity: u32,
    max_msg_len: u32,
    timeout: u64,
    tick_unit: TickUnit,
) {
    let creation_time = tick_unit.nix_time();
    do_write_header(
        header,
        producer_id,
        channel_id,
        capacity,
        max_msg_len,
        timeout,
        tick_unit,
        creation_time,
    );
}
#[inline]
fn do_write_header(
    header: &mut [u8],
    producer_id: u64,
    channel_id: u64,
    capacity: u32,
    max_msg_len: u32,
    timeout: u64,
    tick_unit: TickUnit,
    creation_time: u64,
) -> usize {
    assert!(header.len() >= HEADER_LEN);
    header[0..8].clone_from_slice(&MAGIC_U64.to_le_bytes());
    let latest_v: u64 = LATEST.into();
    header[8..16].clone_from_slice(&latest_v.to_le_bytes());
    header[16..24].clone_from_slice(&producer_id.to_le_bytes());
    header[24..32].clone_from_slice(&channel_id.to_le_bytes());
    header[32..36].clone_from_slice(&capacity.to_le_bytes());
    header[36..40].clone_from_slice(&max_msg_len.to_le_bytes());
    header[40..48].clone_from_slice(&timeout.to_le_bytes());
    header[48..56].clone_from_slice(&creation_time.to_le_bytes());
    let status = Status::Open(creation_time);
    header[56..64].clone_from_slice(&status.status_id().to_le_bytes());
    header[64] = tick_unit.id();
    header[65..73].clone_from_slice(&creation_time.to_le_bytes()); //status time
    let last = 73;
    for item in header.iter_mut().take(HEADER_LEN).skip(last) {
        *item = 0u8;
    }
    HEADER_LEN
}

#[inline]
fn read_u64(header: &[u8], offset: usize) -> u64 {
    assert!(header.len() >= HEADER_LEN);
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
    assert!(header.len() >= HEADER_LEN);
    assert!(offset + 4 < HEADER_LEN);
    u32::from_le_bytes([
        header[offset],
        header[offset + 1],
        header[offset + 2],
        header[offset + 3],
    ])
}

#[inline]
pub fn magic(header: &[u8]) -> u64 {
    read_u64(header, 0)
}

#[inline]
pub fn version(header: &[u8]) -> Version {
    read_u64(header, 8).into()
}

#[inline]
pub fn channel_id(header: &[u8]) -> u64 {
    read_u64(header, 16)
}

#[inline]
pub fn producer_id(header: &[u8]) -> u64 {
    read_u64(header, 24)
}

#[inline]
pub fn capacity(header: &[u8]) -> u32 {
    read_u32(header, 32)
}
#[inline]
pub fn max_msg_len(header: &[u8]) -> u32 {
    read_u32(header, 36)
}
#[inline]
pub fn prod_timeout(header: &[u8]) -> u64 {
    read_u64(header, 40)
}

#[inline]
pub fn creation_time(header: &[u8]) -> u64 {
    read_u64(header, 48)
}

#[inline]
pub fn status(header: &[u8]) -> Option<Status> {
    Status::from_id_and_ts(read_u64(header, 56), status_time(header))
}

#[inline]
pub fn set_status(header: &mut [u8], new_status: Status) -> Result<Status, String> {
    let status_opt = status(header);
    match status_opt {
        Some(status) => match status {
            Status::Open(_) => {
                header[56..64].clone_from_slice(&new_status.status_id().to_le_bytes());
                header[64..72].clone_from_slice(&new_status.timestamp().to_le_bytes());
                Ok(new_status)
            }
            _ => Err(format!(
                "Invalid request. Transition from {} _ > {}",
                status, new_status
            )),
        },
        None => Err("Unknown curent status".to_string()),
    }
}

#[inline]
pub fn status_time(header: &[u8]) -> u64 {
    read_u64(header, 65)
}

#[inline]
pub fn tick_unit(header: &[u8]) -> TickUnit {
    TickUnit::from_id(header[64])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_header() {
        let mut data = vec![0u8; HEADER_LEN];
        let producer_id: u64 = 111;
        let channel_id: u64 = 101;
        let capacity: u32 = 10001;
        let max_msg_len: u32 = 100;
        let timeout: u64 = 10000;
        let tick_unit = TickUnit::Nanos;
        let creation_time: u64 = 1111111;
        let res = do_write_header(
            &mut data,
            channel_id,
            producer_id,
            capacity,
            max_msg_len,
            timeout,
            tick_unit,
            creation_time,
        );
        assert!(res == HEADER_LEN);
        assert!(magic(&data) == MAGIC_U64);
        assert!(version(&data) == LATEST);
        assert!(super::channel_id(&data) == channel_id);
        assert!(super::producer_id(&data) == producer_id);
        assert!(super::capacity(&data) == capacity);
        assert!(super::max_msg_len(&data) == max_msg_len);
        assert!(super::creation_time(&data) == creation_time);
        assert!(super::status_time(&data) == creation_time);
        assert!(status(&data).unwrap() == Status::Open(creation_time));
        let close_ts = creation_time + 10000;
        assert!(
            super::set_status(&mut data, Status::Closed(close_ts)).unwrap()
                == Status::Closed(close_ts)
        );
    }
}
