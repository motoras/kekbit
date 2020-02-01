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
static HEARTBEAT_MSG: &[u8] = &[];

pub trait Reader {
    fn read(&mut self, handler: &mut impl FnMut(&[u8]) -> (), message_count: u16) -> Result<u32, ReadError>;
}

pub trait Writer {
    fn write(&mut self, data: &[u8], len: u32) -> Result<u32, WriteError>;
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
    #[inline(always)]
    fn heartbeat(&mut self) -> Result<u32, WriteError> {
        self.write(HEARTBEAT_MSG, 0)
    }
}
