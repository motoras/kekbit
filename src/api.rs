//! Defines the general kekbit access protocol, based on the [Reader](api/trait.Reader.html) and [Writer](api/trait.Writer.html) traits.
use std::io::Error;
use std::io::Write;

///An entity which can be written into a channel
pub trait Encodable {
    /// Encodes an object into a `Write`. It could simply write the
    /// raw binary representation of the data, or it could use some
    /// well known data such JSON on Bincode.
    ///
    /// # Arguments
    ///
    /// * `write` - A Writer used to push data into channel
    ///
    /// # Errors
    ///
    /// If the encoding fails or an IO error occurs and the operation is cancelled.
    fn encode(&self, write: &mut impl Write) -> Result<usize, Error>;
}

///Any binary data is ready to be encoded into a channel.
impl<T: AsRef<[u8]>> Encodable for T {
    #[inline]
    fn encode(&self, w: &mut impl Write) -> Result<usize, Error> {
        w.write(self.as_ref())
    }
}
/// Handlers are components which will decorate a *write operation* .
/// They can be use to add various metadata to a record(like timestamp, sequence id,
/// universal unique id, check sum, record encoding type) either before or after
/// a record was pushed into channel or to transform or even replace a record with
/// a different one before it is pushed into a channel.
///
/// Handlers are composable by design so it is expected that multiple handlers will be chained
/// together and used to process a record.
///
/// A handler will usually implement one or both of the `incoming/outgoing` methods.
/// Most of the Handlers will leave the `handle` method unchanged. Metahandlers
/// (handlers that compose other handlers) such handler chains or basic handlers which
/// can be used directly or expect to be at the bottom of a handlers chain may implement
/// the hanlde method.
pub trait Handler {
    /// Action to be done *before* a record is pushed into channel.
    /// Most common handlers will override this method, in order to add some header to a given record,
    /// or to transform a record before is written into the channel.
    ///
    /// # Arguments
    ///
    /// * `_data` - Data to push data into channel
    /// * `_write` - Write interface to channel
    ///
    /// # Errors
    ///
    /// If this method tries to write some data in the channel and the operation fails.
    /// If the call fails no other handlers will be called and the write action will be aborted.
    #[inline]
    fn incoming(&mut self, _data: &impl Encodable, _write: &mut impl Write) -> Result<usize, Error> {
        Ok(0)
    }

    /// Action to be done *after* a record is pushed into channel.
    ///
    /// # Arguments
    ///
    /// * `_data` - Data to push data into channel
    /// * `_write` - Write interface to channel
    ///
    /// # Errors
    ///
    /// If this method tries to write some data in the channel and the operation fails.
    /// If the call fails no other handlers will be called and the write action will be aborted.
    #[inline]
    fn outgoing(&mut self, _data: &impl Encodable, _write: &mut impl Write) -> Result<usize, Error> {
        Ok(0)
    }

    /// Action to be done by this handler. By default this method will chain the `incoming` and  the `outgoing`
    /// methods. Complex handlers may override this method to chain multiple handlers together.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to push data into channel
    /// * `write` - Write interface to channel    
    ///
    /// # Errors
    ///
    /// If this method tries to write some data in the channel and the operation fails.
    /// If the call fails no other handlers will be called and the write action will be aborted.
    #[inline]
    fn handle(&mut self, data: &impl Encodable, w: &mut impl Write) -> Result<usize, Error> {
        self.incoming(data, w).and_then(|_| self.outgoing(data, w))
    }
}

/// The simplest and most important of all handlers. Just writes data into channel.
/// If no data processing is required before the write operation, this handler is
/// expected to be at the bottom of a handler chain. Also this is the perfect handler
/// to use for the simplest of channels, the ones which do not want to append any metadata
/// to a given record.
#[derive(Default)]
pub struct EncoderHandler {}
impl Handler for EncoderHandler {
    /// Writes the given encodable data in to a channel.
    #[inline]
    fn handle(&mut self, data: &impl Encodable, w: &mut impl Write) -> Result<usize, Error> {
        data.encode(w)
    }
}

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
/// multiple writers may cooperate during the writing process.
pub trait Writer {
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
    fn write<E: Encodable>(&mut self, data: &E) -> Result<u32, WriteError>;
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
    Failed,
    ///Writer timeout had been detected. While the writer may resume pushing data into the channel, most likely he had abandoned the channel.
    ///It holds the last time stamp at which the channel was still valid.
    Timeout(u64),
    ///Channel is closed no more data will be pushed into.
    Closed,
    ///Channel full. There is no more space available in this channel.
    ChannelFull,
}

///The `Reader` trait allows reading bytes from a kekbit channel. Implementers of this trait
/// are called 'kekbit readers'. Usually a reader is bound to a given channel, and it is
/// expected that multiple readers will safely access the same channel simultaneous.
pub trait Reader {
    /// Attempts to read a message from the channel without blocking.
    /// This method will either read a message from the channel immediately or return if no data is available.
    ///     
    /// Returns the next message available from the channel, if there is one, None otherwise.
    ///
    /// # Errors
    /// Various [errors](enum.ReadError.html) may occur such: a `writer` timeout is detected, end of channel is reached, channel is closed or channel data is corrupted.
    /// Once an error occurs the channel is marked as exhausted so *any future read operation will fail*.
    ///
    fn try_read<'a>(&mut self) -> Result<Option<&'a [u8]>, ReadError>;

    /// Checks if the channel have been exhausted or is still active.  If the channel is active, a future read operation
    /// may or may not succeed but it should be tried. No data will ever come from an exhausted channel,
    /// any read operation is futile.
    ///
    /// Returns `None` if the channel is active, or `Some<ReadError>` if the channel has been exhausted.
    /// The error returned is the reason for which the channel is considered exhausted.
    fn exhausted(&self) -> Option<ReadError>;
}
