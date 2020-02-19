use crate::codecs::DataFormat;
use crate::codecs::Decodable;
use crate::codecs::Encodable;
use std::io::Result;
use std::io::Write;

const ID: u64 = 2;
const MEDIA_TYPE: &str = "application/octet-stream";

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

impl<T: AsRef<[u8]>> Encodable<RawBinDataFormat> for T {
    #[inline]
    fn encode(&self, _format: &RawBinDataFormat, w: &mut impl Write) -> Result<usize> {
        w.write(self.as_ref())
    }
}

impl<'a, T: From<&'a [u8]>> Decodable<'a, RawBinDataFormat, T> for T {
    #[inline]
    fn decode(_d: &RawBinDataFormat, data: &'a [u8]) -> Result<T> {
        Ok(data.into())
    }
}

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
        msg.encode(&df, &mut cursor).unwrap();
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

    #[test]
    fn encode_decode() {
        let mut vec = Vec::<u8>::new();
        let mut cursor = Cursor::new(&mut vec);
        let enc_msg = &[1u8; 10][..];
        let df = RawBinDataFormat;
        enc_msg.encode(&df, &mut cursor).unwrap();
        let dec_msg = &Vec::decode(&df, &vec).unwrap()[..];
        assert_eq!(enc_msg, dec_msg);
    }
}
