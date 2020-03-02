use crate::api::RecordHeader;
use crate::core::TickUnit;
use std::io::Result;
use std::io::Write;

#[derive(Default)]
pub struct NoRecHeader {}

impl RecordHeader for NoRecHeader {
    #[inline]
    fn apply(&mut self, _w: &mut impl Write) -> Result<usize> {
        Ok(0)
    }
}

pub struct TimestampHeader {
    tick: TickUnit,
}

impl RecordHeader for TimestampHeader {
    #[inline]
    fn apply(&mut self, w: &mut impl Write) -> Result<usize> {
        w.write(&self.tick.nix_time().to_le_bytes())
    }
}

pub struct SequenceHeader {
    seq: u64,
}

impl RecordHeader for SequenceHeader {
    #[inline]
    fn apply(&mut self, w: &mut impl Write) -> Result<usize> {
        self.seq += 1;
        w.write(&self.seq.to_le_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ts() {
        // let s = "ABCDEF";
        // let ts = 1234567890;
        // let tse = TsEncodable {
        //     encodable: &s,
        //     timestamp: ts,
        // };
        // println!("{:?}", tse.timestamp);
    }
}
