use crate::codecs::DataFormat;
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
    fn media_type() -> &'static str {
        MEDIA_TYPE
    }
}

impl Encodable<PlainTextDataFormat> for String {
    #[inline]
    fn encode_to(&self, _format: &PlainTextDataFormat, w: &mut impl Write) -> Result<usize> {
        w.write(self.as_bytes())
    }
}

impl<'a> Encodable<PlainTextDataFormat> for &'a str {
    #[inline]
    fn encode_to(&self, _format: &PlainTextDataFormat, w: &mut impl Write) -> Result<usize> {
        w.write(self.as_bytes())
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
}
