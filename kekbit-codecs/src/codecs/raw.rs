use crate::codecs::DataFormat;
use crate::codecs::Encodable;
use std::io::Write;

const ID: u64 = 2;
const MEDIA_TYPE: &'static str = "application/octet-stream";

/// The most basic data format. It just simply writes raw bytes into the channel, without
/// any regard of the underlying data's structure
pub struct RawBinDataFormat;
impl DataFormat for RawBinDataFormat {
    ///Returns 2, the id of the most basic encoder.
    #[inline]
    fn id() -> u64 {
        ID
    }
    ///Returns "application/octet-stream"
    #[inline]
    fn media_type() -> &'static str {
        MEDIA_TYPE
    }
}

impl<'a> Encodable<RawBinDataFormat> for &'a [u8] {
    #[inline]
    fn encode_to(&self, _format: &RawBinDataFormat, w: &mut impl Write) {
        w.write(self).unwrap();
    }
}

//Will see if this is need it or just a basic nice to have
//use std::io::Read;
// impl<'a> Encodable<RawBinDataFormat> for &'a Read {
//     fn encode_to(&mut self, _format: &RawBinDataFormat, w: &mut impl Write) {
//         std::io::copy(&mut self, w).unwrap();
//     }
// }

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;

    #[test]
    fn check_raw_binary_encoder_slice() {
        let mut vec = Vec::<u8>::new();
        let mut cursor = Cursor::new(&mut vec);
        let msg = &[1u8; 10][..];
        let df = RawBinDataFormat;
        msg.encode_to(&df, &mut cursor);
        assert_eq!(cursor.position() as usize, msg.len());
        cursor.set_position(0);
        let expected = &mut [11u8; 10][..];
        cursor.read_exact(expected).unwrap();
        assert_eq!(expected, msg);
    }
    #[test]
    fn check_data_format() {
        assert_eq!(RawBinDataFormat::id(), ID);
        assert_eq!(RawBinDataFormat::media_type(), MEDIA_TYPE);
    }
}
