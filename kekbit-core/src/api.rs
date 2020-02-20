//! Defines read and write operations for a kekbit channel.
use kekbit_codecs::codecs::DataFormat;
use kekbit_codecs::codecs::Encodable;
use std::io::Error;
///Channel Access errors
#[derive(Debug)]
pub enum ChannelError {
    ///The channel has an invalid signature. The channel signature must be `0x2A54_4942_4B45_4B2A`
    InvalidSignature {
        ///The expected signature always `0x2A54_4942_4B45_4B2A`
        expected: u64,
        ///The signature red from the kekbit storage
        actual: u64,
    },
    ///The channel's storage is of an incompatible file format
    IncompatibleVersion {
        ///Expected storage version
        expected: u64,
        ///Actual  storage version
        actual: u64,
    },
    ///The channel's capacity is invalid. Either too small or is not aligned to 8 bytes.
    InvalidCapacity {
        ///Actual capacity
        capacity: u32,
        ///Reason why the capacity is invalid
        msg: &'static str,
    },
    ///The maximum message length specified is invalid
    InvalidMaxMessageLength {
        ///The specified maximum message length
        msg_len: u32,
        ///Reason why maximum message length is invalid
        msg: &'static str,
    },
    ///The channel storage does not exist
    StorageNotFound {
        ///The file expected to back the channel storage
        file_name: String,
    },

    ///The channel storage is not ready to access
    StorageNotReady {
        ///The file that backs the channel storage
        file_name: String,
    },
    ///The channel storage is not ready to access
    StorageAlreadyExists {
        ///The file that backs the channel storage
        file_name: String,
    },
    ///The channel storage can't be accessed
    CouldNotAccessStorage {
        ///The file that backs the channel storage
        file_name: String,
    },
    ///Mapping the channel's file to memory had failed
    MemoryMappingFailed {
        reason: String,
    },

    AccessError {
        reason: String,
    },
}

///Write operation errors
#[derive(Debug)]
pub enum WriteError {
    ///There is not enough space available in the channel for any write. The channel is full.
    ChannelFull,
    /// The record was larger than the maximum allowed size or the maximum available space.
    NoSpaceForRecord,
    /// The encoding operation had failed
    EncodingError(Error),
}

///The `Writer` trait allows writing chunk of bytes as records into a kekbit channel.
/// Implementers of this trait are called 'kekbit writers'. Usually a writer is bound to
/// a given channel, and it is expected that there is only one writer which directly writes into the channel, however
/// multiple writers may cooperate during the writing process. For any given channel a  [DataFormat](../codecs/trait.DataFormat.html) must be specified.
pub trait Writer<D: DataFormat> {
    /// Writes a given record to a kekbit channel.
    ///
    /// Returns the total amount of bytes wrote into the channel or a `WriteError` if the write operation fails.
    ///
    /// # Arguments
    ///
    /// * `data` - information to be encoded and pushed into channel.
    ///
    /// # Errors
    ///
    /// If the operation fails, than an error variant will be returned. Some errors such [EncodingError or NoSpaceForRecord](enum.WriteError.html) may
    /// allow future writes to succeed while others such [ChannelFull](enum.WriteError.html#ChannelFull) signals the end of life for the channel.
    fn write(&mut self, data: &impl Encodable<D>) -> Result<u32, WriteError>;
    /// Writes into the stream a heartbeat message. This method shall be used by all writers
    /// which want to respect to timeout interval associated to a channel. Hearbeating is the
    /// expected mechanism by which a channel writer will keep the active readers interested in
    /// the data published on the channel.
    /// Heartbeat shall be done regularly at a time interval which ensures that at least one heartbeat
    /// is sent between any two 'timeout' long intervals.
    ///
    /// Returns the total amount of bytes wrote into the channel or a `WriteError` if the write operation fails.
    ///
    /// # Errors
    ///
    /// If this call fails than an error variant will be returned. The errors are not recoverable,
    /// they signal that the channel had reached the end of its lifetime.
    fn heartbeat(&mut self) -> Result<u32, WriteError>;

    /// Flushes the stream which possibly backs the kekbit writer.
    /// By default this method does nothing, and should be implemented only for `Writer`s which it makes sense.
    /// Returns the success of the operation
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

///Read operation errors
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReadError {
    ///Read operation had unexpectedly failed. Usually will happen when a channel was corrupted.
    Failed {
        ///The amount of bytes read *before* the error occurred.
        bytes_read: u32,
    },
    ///Writer timeout had been detected. While the writer may resume pushing data in to the channel, most likely he had abandoned the channel.
    Timeout {
        ///Last time stamp at which the channel was still considered valid.
        timeout: u64,
    },
    ///Channel is closed no more data will be pushed into.
    Closed {
        ///The amount of bytes read *before* the channel close mark was reached.
        bytes_read: u32,
    },
    ///Channel full. There is no more space available in this channel.
    ChannelFull {
        ///The amount of bytes read *before* the end of channel was reached.
        bytes_read: u32,
    },
}

impl ReadError {
    ///Returns the number of valid bytes read before an error occurred.
    pub fn bytes_read(&self) -> u32 {
        match self {
            ReadError::Timeout { .. } => 0,
            ReadError::Closed { bytes_read } => *bytes_read,
            ReadError::ChannelFull { bytes_read } => *bytes_read,
            ReadError::Failed { bytes_read } => *bytes_read,
        }
    }
}

///The `Reader` trait allows reading bytes from a kekbit channel. Implementers of this trait
/// are called 'kekbit readers'. Usually a reader is bound to a given channel, and it is
/// expected that multiple readers will safely access the same channel simultaneous.
pub trait Reader {
    ///Attempts to read a message from the channel without blocking.
    ///This method will either read a message from the channel immediately or return if no data is available.
    ///     
    /// Returns the next message available from the channel, if there is one, None otherwise.
    ///
    /// # Errors
    /// Various [errors](enum.ReadError.html) may occur such: a `writer` timeout is detected, end of channel is reached, channel is closed or channel data is corrupted.
    /// Once an error occurs, *any future read operation will fail*, so no more other records could ever be read from this channel.
    ///
    fn try_read<'a>(&mut self) -> Result<Option<&'a [u8]>, ReadError>;
}
