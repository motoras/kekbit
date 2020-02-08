//! Time granularity units used in kekbit.
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

///A TickUnit represents a specific time duration but does not maintain time information, it only helps define the time granularity
///required to used in various contexts by all kekbit components which *share a given channel*.
///For each channel it's TickUnit will be spcified at creation and will *never be changed*
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TickUnit {
    ///TickUnit representing one thousandth of a microsecond.
    Nanos,
    ///TickUnit representing one thousandth of a millisecond.
    Micros,
    ///TickUnit representing one thousandth of a second.
    Millis,
    ///TickUnit representing one second.
    Secs,
}

impl TickUnit {
    ///Returns the unique u8 id assigned to every TickUnit. This id it's used for serialization it would never change.
    ///
    /// # Examples
    /// ```
    /// use kekbit_core::tick::TickUnit::*;
    ///
    /// assert_eq!(Nanos.id(), 9);
    /// assert_eq!(Micros.id(), 6);
    /// assert_eq!(Millis.id(), 3);
    /// assert_eq!(Secs.id(), 0);
    /// ```
    #[inline]
    pub fn id(self) -> u8 {
        match self {
            TickUnit::Nanos => 9,
            TickUnit::Micros => 6,
            TickUnit::Millis => 3,
            TickUnit::Secs => 0,
        }
    }

    /// Returns the tick unit with the given id
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the tick unit as u8. In the curent kekbit version it must be **0, 3, 6 or 9**
    ///
    /// # Panics
    ///
    /// If the specified id has no tick unit attached
    ///
    #[inline]
    pub fn from_id(id: u8) -> TickUnit {
        match id {
            9 => TickUnit::Nanos,
            6 => TickUnit::Micros,
            3 => TickUnit::Millis,
            0 => TickUnit::Secs,
            _ => panic!("Unknown time unit id {}", id),
        }
    }
    /// Returns the total number of tick units contained by this `Duration` as a u64.
    /// If the tiemstamp size is longer than 64 bits, it will be truncated to the lower 64 bits
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use kekbit_core::tick::TickUnit::*;
    ///
    ///let duration = Duration::new(1, 500_000_000); //1 sec and a half
    ///assert_eq!(Nanos.convert(duration), 1_500_000_000);
    ///assert_eq!(Micros.convert(duration), 1_500_000);
    ///assert_eq!(Millis.convert(duration), 1_500);
    ///assert_eq!(Secs.convert(duration), 1);
    /// ```
    #[inline]
    pub fn convert(self, duration: Duration) -> u64 {
        match self {
            TickUnit::Nanos => duration.as_nanos() as u64,
            TickUnit::Micros => duration.as_micros() as u64,
            TickUnit::Millis => duration.as_millis() as u64,
            TickUnit::Secs => duration.as_secs(),
        }
    }
    ///Returns the difference, measured in the current tick unit, between the current time and midnight, January 1, 1970 UTC.
    ///
    /// # Examples
    ///  ```
    /// use kekbit_core::tick::TickUnit::*;
    ///
    /// println!("{}ms since January 1, 1970 UTC", Millis.nix_time());
    /// ```
    #[inline]
    pub fn nix_time(self) -> u64 {
        self.convert(SystemTime::now().duration_since(UNIX_EPOCH).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::TickUnit::*;
    use super::*;
    use std::time::Duration;
    #[test]
    fn check_ids_symetry() {
        assert_eq!(Nanos.id(), TickUnit::from_id(Nanos.id()).id());
        assert_eq!(Micros.id(), TickUnit::from_id(Micros.id()).id());
        assert_eq!(Millis.id(), TickUnit::from_id(Millis.id()).id());
        assert_eq!(Secs.id(), TickUnit::from_id(Secs.id()).id());
    }

    #[test]
    #[should_panic]
    fn check_wrong_id() {
        TickUnit::from_id(123);
    }

    #[test]
    fn test_coversion() {
        let duration = Duration::new(1, 500_000_000); //1 sec and a half
        assert_eq!(Nanos.convert(duration), 1_500_000_000);
        assert_eq!(Micros.convert(duration), 1_500_000);
        assert_eq!(Millis.convert(duration), 1_500);
        assert_eq!(Secs.convert(duration), 1);
    }

    #[test]
    fn check_ids() {
        assert_eq!(Nanos.id(), 9);
        assert_eq!(Micros.id(), 6);
        assert_eq!(Millis.id(), 3);
        assert_eq!(Secs.id(), 0);
    }

    #[test]
    fn check_nix_time() {
        let t1 = Nanos.nix_time();
        let t2 = Nanos.nix_time();
        assert!(t1 <= t2);
    }
}
