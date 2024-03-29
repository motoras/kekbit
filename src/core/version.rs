use std::fmt::{Display, Formatter, Result};
use std::ops::Deref;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
#[repr(transparent)]
pub(crate) struct Version {
    version: u64,
}

impl Version {
    #[inline]
    #[allow(dead_code)]
    fn new(major: u16, minor: u16, patch: u32) -> Self {
        let mut v_u64 = 0u64;
        v_u64 |= u64::from(major) << 48;
        v_u64 |= u64::from(minor) << 32;
        v_u64 |= u64::from(patch);
        Version { version: v_u64 }
    }

    #[inline]
    pub fn major(self) -> u16 {
        (self.version >> 48) as u16
    }

    #[inline]
    pub fn minor(self) -> u16 {
        ((self.version << 16) >> 48) as u16
    }

    #[inline]
    pub fn patch(self) -> u32 {
        (self.version & 0x0000_0000_FFFF_FFFF) as u32
    }

    #[inline]
    pub fn is_compatible(self, other: Version) -> bool {
        self >= other
    }

    #[inline]
    pub fn latest() -> Version {
        Version::new(1, 0, 0)
    }
}

// impl Into<u64> for Version {
//     #[inline]
//     fn into(self) -> u64 {
//         self.version
//     }
// }

impl From<Version> for u64 {
    fn from(version: Version) -> Self {
        version.version
    }
}

impl From<u64> for Version {
    fn from(version: u64) -> Self {
        Version { version }
    }
}

impl Deref for Version {
    type Target = u64;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.version
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

#[cfg(test)]
mod tests {
    use super::Version;
    #[test]
    fn check_version_creation() {
        for i in 1..1024 {
            let v = Version::new(i, i, i as u32);
            assert!(v.major() == i as u16);
            assert!(v.minor() == i as u16);
            assert!(v.patch() == i as u32);
            let v_u64: u64 = v.into();
            assert!(v_u64 == *v);
            assert!(format!("{}", v) == format!("{}.{}.{}", i, i, i));
        }
    }

    #[test]
    fn check_conversion() {
        let v1 = Version::new(1, 2, 3);
        let v_u64: u64 = v1.into();
        let v2 = Version::from(v_u64);
        assert_eq!(v1, v2);
        let v3: Version = v_u64.into();
        assert_eq!(v1, v3);
    }
}
