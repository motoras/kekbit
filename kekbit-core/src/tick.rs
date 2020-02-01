use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TickUnit {
    Nanos,
    Micros,
    Millis,
    Secs,
}

impl TickUnit {
    #[inline]
    pub fn from_id(id: u8) -> TickUnit {
        match id {
            9 => TickUnit::Nanos,
            6 => TickUnit::Micros,
            3 => TickUnit::Millis,
            1 => TickUnit::Secs,
            _ => panic!("Unknown time unit id {}", id),
        }
    }
    #[inline]
    pub fn id(&self) -> u8 {
        match self {
            TickUnit::Nanos => 9,
            TickUnit::Micros => 6,
            TickUnit::Millis => 3,
            TickUnit::Secs => 0,
        }
    }

    #[inline]
    pub fn convert(&self, duration: Duration) -> u64 {
        match self {
            TickUnit::Nanos => duration.as_nanos() as u64,
            TickUnit::Micros => duration.as_micros() as u64,
            TickUnit::Millis => duration.as_millis() as u64,
            TickUnit::Secs => duration.as_secs(),
        }
    }

    #[inline]
    pub fn nix_time(&self) -> u64 {
        self.convert(SystemTime::now().duration_since(UNIX_EPOCH).unwrap())
    }
}
