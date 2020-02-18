use std::io::Write;
///A data format that can be use by a kekbit channel
pub trait DataFormat {
    ///Returns the unique identifer of this data format. As a convention `'standard`
    /// data formats must have an id below 2^32, while application specific formats
    /// should have an id equal or greater with 2^32
    fn id() -> u64;
    /// Returns the media type associated with this codec. This value is just informative.
    /// E.g for a Json encoder should return "`application/json`
    fn media_type() -> &'static str;
}

///An entity which can be written in a channel using the specified data format
pub trait Encodable<D: DataFormat> {
    fn encode_to(&self, d: &D, w: &mut impl Write) -> std::io::Result<usize>;
}

// struct JsonDataFormat;

// impl DataFormat for JsonDataFormat {
//     fn id() -> u64 {
//         17
//     }
//     fn media_type() -> &'static str {
//         "application/json"
//     }
// }

// impl<T: Serialize> Encodable<JsonDataFormat> for T {
//     fn encode_to(&self, _format: &JsonDataFormat, w: &mut impl Write) {
//         to_writer(w, self).unwrap();
//     }
// }

pub mod raw;
pub mod text;
