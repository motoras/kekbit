//! Defines read and write operations for a kekbit channel.

///Write operation errors
#[derive(Debug, Copy, Clone)]
pub enum WriteError {
    /// There is not enough space available in the channel for such an operation.
    NoSpaceAvailable {
        ///The space amount required by the record. It will be larger than the record size.
        required: u32,
        ///The space amount available in the channel.
        left: u32,
    },
    /// The record was longer than the maximum allowed.
    MaxRecordLenExceed {
        ///The size of the record to be written.
        rec_len: u32,
        ///The maximum allowed size for a record.
        max_allowed: u32,
    },
}

static HEARTBEAT_MSG: &[u8] = &[];

///The `Writer` trait allows writing chunk of bytes as records into a kekbit channel.
/// Implementors of this trait are called 'kekbit writers'. Usually a writer is bound to
/// a given channel, and it is expected that there is only one writer which directly writes into the channel, however
/// multiple writers may cooperate during the writting process.
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
    /// If this function encounts any error, than an error variant will be returned.
    /// Regardless the error varaint a future write with a smaller record size may be sucessful.
    ///
    fn write(&mut self, data: &[u8], len: u32) -> Result<u32, WriteError>;
    /// Writes into the stream a heartbeat message. This method shall be used by all writers
    /// which want to respect to timeout interval associated to a channel. Hearbeating is the
    /// expected mechanism by which a channel writer will keep the active readers interested in
    /// the data published on the channel.
    /// Heartbeat shall be done regulary, at a time interval which ensures that at least one heartbeat
    /// is sent between any two 'timeout' long intervals.
    ///
    /// Returns the total amount of bytes wrote into the channel or a `WriteError` if the write operation fails.
    ///
    /// # Errors
    ///
    /// If this function encounts any error, than an error variant will be returned. However
    /// in this case the erros are not recovarable they signal tha the channel is at the
    /// end of its lifetime.
    #[inline(always)]
    fn heartbeat(&mut self) -> Result<u32, WriteError> {
        self.write(HEARTBEAT_MSG, 0)
    }

    /// Flushes the stream which possibly backs the kekbit writer
    /// By default this method does nothing, and should be implemented only for `Writer`s which
    /// are backed by a file or a network stream.
    /// Returns the success of the oepration
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

///Read operation errors
#[derive(Debug, Copy, Clone)]
pub enum ReadError {
    ///Read operation had unexpectedly failed. Usualy will happen when a channel was corrupted.
    Failed {
        ///The amount of bytes read *before* the error occured.
        bytes_read: u32,
    },
    ///Writer timeout had been detected. While the writer may resume pushing data in to the channel, most likely he had abandoned the channel.
    Timeout {
        ///Last timestamp at which the channel was still considered valid.
        timeout: u64,
    },
    ///Channel is closed no more data will be pushed into.
    Closed {
        ///The amount of bytes read *before* the channel close mark was reached.
        bytes_read: u32,
    },
    ///End of Channel reached. There si no more space available in this channel.
    EndOfChannel {
        ///The amount of bytes read *before* the end of channel was reached.
        bytes_read: u32,
    },
}

impl ReadError {
    ///Returns the number of valid bytes read before an error occured.
    pub fn bytes_read(&self) -> u32 {
        match self {
            ReadError::Timeout { .. } => 0,
            ReadError::Closed { bytes_read } => *bytes_read,
            ReadError::EndOfChannel { bytes_read } => *bytes_read,
            ReadError::Failed { bytes_read } => *bytes_read,
        }
    }
}

///The `Reader` trait allows reading bytes from a kekbit channel. Implementors of this trait
/// are called 'kekbit readers'. Usually a reader is bound to a given channel, and it is
/// expected that multiple readers will safely access the same channel simultanious.
pub trait Reader {
    ///Acceses a number of records from the kekbit channel, and for each record, calls
    ///the given callback.
    ///
    /// Returns the amount of bytes read from the channel
    ///
    /// # Arguments
    ///
    /// * `handler` - The callback function to be called when a record is pulled from the channel
    /// * `message_count` - A hint about how many records shall be read from the channel before the method completes.
    ///                     It is expected that this metohd will take from the channel at most this many records
    ///
    ///
    /// # Errors
    ///
    /// If this function encounts any error, than an error variant will be returned. These
    /// errors are not expected to be recovarable. Once any error except `Timeout` occured, there will never be
    /// data to read pass the current read marker. However reading from begining of th channel to the current
    /// read marker should still be a valid operation. The `Timeout` exception,
    /// may or may not be recovarable, depends on the channel `Writer` behaviour.
    ///
    fn read(&mut self, handler: &mut impl FnMut(&[u8]) -> (), message_count: u16) -> Result<u32, ReadError>;
}
