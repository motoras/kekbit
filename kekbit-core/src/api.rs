#[derive(Debug)]
pub enum WriteError {
    Closed,
    ChannelTimeout { expired: u64, crt_timestamp: u64 },
    NoSpaceAvailable { required: u32, left: u32 },
    MaxRecordLenExceed { rec_len: u32, max_allowed: u32 },
}

#[derive(Debug)]
pub enum ReadError {
    Failed { bytes_read: u32 },
    Timeout { bytes_read: u32, timeout: u64 },
    Closed { bytes_read: u32 },
    EndOfChannel { bytes_read: u32 },
}

impl ReadError {
    pub fn bytes_read(&self) -> u32 {
        match self {
            ReadError::Timeout { bytes_read, .. } => *bytes_read,
            ReadError::Closed { bytes_read } => *bytes_read,
            ReadError::EndOfChannel { bytes_read } => *bytes_read,
            ReadError::Failed { bytes_read } => *bytes_read,
        }
    }
}
