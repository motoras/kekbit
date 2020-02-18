//! Defines read and write operations for a kekbit channel.

///Channel Access errors
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteError {
    ///There is not enough space available in the channel for any write. The channel is full.
    ChannelFull,
    NoSpaceAvailable {
        ///The space amount required by the record. It will be larger than the record size.
        required: u32,
        ///The space amount available in the channel.
        left: u32,
    },
    /// The record was larger than the maximum allowed size or the maximum available space.
    NoSpaceForRecord,
    /// The record was longer than the maximum allowed.
    MaxRecordLenExceed {
        ///The size of the record to be written.
        rec_len: u32,
        ///The maximum allowed size for a record.
        max_allowed: u32,
    },
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
    /// * `data` - The buffer which contains the record data to be written.
    /// * `len` - The amount of data to be write in the channel, the record length.
    ///
    /// # Errors
    ///
    /// If the operation fails, than an error variant will be returned.
    /// Regardless the error variant a future write with a smaller record size may be successful.
    ///
    fn write(&mut self, data: &[u8], len: u32) -> Result<u32, WriteError>;
    /// Writes into the stream a heartbeat message. This method shall be used by all writers
    /// which want to respect to timeout interval associated to a channel. Hearbeating is the
    /// expected mechanism by which a channel writer will keep the active readers interested in
    /// the data published on the channel.
    /// Heartbeat shall be done regularly, at a time interval which ensures that at least one heartbeat
    /// is sent between any two 'timeout' long intervals.
    ///
    /// Returns the total amount of bytes wrote into the channel or a `WriteError` if the write operation fails.
    ///
    /// # Errors
    ///
    /// If this call fails, than an error variant will be returned. However
    /// in this case the errors are not recoverable, they signal that the channel is at the
    /// end of its lifetime.    
    fn heartbeat(&mut self) -> Result<u32, WriteError>;

    /// Flushes the stream which possibly backs the kekbit writer.
    /// By default this method does nothing, and should be implemented only for `Writer`s which
    /// it makes sense.
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
///Errors caused by failed [move_to](trait.Reader.html#method.move_to) operation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum InvalidPosition {
    ///Position is not properly aligned with the channel's records
    Unaligned { position: u32 },
    ///Position is not available  in the channel - it is past the last valid record of the channel.
    Unavailable { position: u32 },
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
    ///Accesses a number of records from the kekbit channel, and for each record, calls
    ///the given callback.
    ///
    /// Returns the amount of bytes read from the channel
    ///
    /// # Arguments
    ///
    /// * `handler` - The callback function to be called when a record is pulled from the channel.
    ///               The function will receive as parameters the position of the message in the channel, and the message in binary format.
    /// * `message_count` - A hint about how many records shall be read from the channel before the method completes.
    ///                     It is expected that this method will take from the channel at most this many records
    ///
    ///
    /// # Errors
    ///
    /// If this function fails, than an error variant will be returned. These errors are not expected to be recoverable. Once any error except `Timeout` occurred, there will never be
    /// data to read pass the current read marker. However reading from beginning of the channel to the current
    /// read marker should still be a valid operation. The `Timeout` exception,
    /// may or may not be recoverable, depends on the channel `Writer` behaviour.
    fn read(&mut self, handler: &mut impl FnMut(u32, &[u8]) -> (), message_count: u16) -> Result<u32, ReadError>;

    /// Moves the reader to the given position in the channel *if the position is valid and points
    /// to the beginning of a record*. This method could be used by a reader to resume work from
    /// a previous session.
    ///
    /// Returns the position if the operation succeeds
    ///
    /// # Arguments
    ///
    /// * `position` - The position in channel where we want the reader to point. The value is accounted
    ///                 from the beginning of the channel(e.g. a position of zero means the beginning of the channel). The position
    ///                 must be valid, it must be properly aligned, and is should point to the start of a record.
    ///
    /// #  Errors
    ///
    /// If the channel is corrupted or the position is invalid a [InvalidPosition](enum.InvalidPosition.html)
    /// will occur.
    ///
    fn move_to(&mut self, position: u32) -> Result<u32, InvalidPosition>;
}
