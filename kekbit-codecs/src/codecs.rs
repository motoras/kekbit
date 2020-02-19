use std::io::Result;
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
    ///Encodes an object on the spcified data format into a `Write`
    ///
    /// # Errors
    ///
    /// If the encoding fails or an IO erorr occurs.
    fn encode(&self, d: &D, w: &mut impl Write) -> Result<usize>;
}

///Any type wich can be decoded from a u8 slice in the specified data format
pub trait Decodable<'a, D: DataFormat, T> {
    ///Decodes a byte slice using the specified data format
    ///
    /// # Errors
    ///
    /// If the decodign fails an error will be returned
    fn decode(d: &D, data: &'a [u8]) -> Result<T>;
}

//TODO decorators such timestamp or id

pub mod raw;
pub mod text;
