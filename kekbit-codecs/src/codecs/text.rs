use crate::codecs::DataFormat;
use crate::codecs::Decodable;
use crate::codecs::Encodable;
use std::io::Result;
use std::io::Write;

const ID: u64 = 3;
const MEDIA_TYPE: &str = "text/plain";

///Simple unstructured text format. Any applications which just want to exchange plain text may used
/// (e.g. a chat clients, or an application used to exchang text files between peers).
pub struct PlainTextDataFormat;
impl DataFormat for PlainTextDataFormat {
    ///Returns three, the id of the most simple text encoder
    #[inline]
    fn id() -> u64 {
        ID
    }
    #[inline]
    //Returns "text/plain";
    fn media_type() -> &'static str {
        MEDIA_TYPE
    }
}

impl<T: AsRef<str>> Encodable<PlainTextDataFormat> for T {
    #[inline]
    fn encode_to(&self, _format: &PlainTextDataFormat, w: &mut impl Write) -> Result<usize> {
        w.write(self.as_ref().as_bytes())
    }
}

impl<'a, T: From<String>> Decodable<'a, PlainTextDataFormat, T> for T {
    #[inline]
    fn decode(_d: &PlainTextDataFormat, data: &'a [u8]) -> Result<T> {
        Ok(String::from_utf8_lossy(data).to_string().into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn check_plain_text_encoder() {
        let mut vec = Vec::<u8>::new();
        let mut cursor = Cursor::new(&mut vec);
        let df = PlainTextDataFormat;
        let msg = "They are who we thought they are";
        msg.encode_to(&df, &mut cursor).unwrap();
        assert_eq!(cursor.position() as usize, msg.len());
        msg.to_string().encode_to(&df, &mut cursor).unwrap();
        assert_eq!(cursor.position() as usize, 2 * msg.len());
    }

    #[test]
    fn check_data_format() {
        assert_eq!(PlainTextDataFormat::id(), ID);
        assert_eq!(PlainTextDataFormat::media_type(), MEDIA_TYPE);
    }

    #[test]
    fn encode_decode() {
        let mut vec = Vec::<u8>::new();
        let mut cursor = Cursor::new(&mut vec);
        let df = PlainTextDataFormat;
        let enc_msg = "They are who we thought they are";
        enc_msg.encode_to(&df, &mut cursor).unwrap();
        let dec_msg = String::decode(&df, &vec).unwrap();
        assert_eq!(enc_msg, dec_msg);
    }
}
