use {
    crate::{error::Error, pool::AsyncPoolTrait, Connection},
    std::{cmp, fmt, future::Future, pin::Pin},
};

pub trait Migration {
    fn name(&self) -> &'static str;
    fn up<'a>(
        &'a self,
        pool: &'a dyn AsyncPoolTrait<Connection>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;
    fn down<'a>(
        &'a self,
        pool: &'a dyn AsyncPoolTrait<Connection>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a>>;
}

pub struct MigrationList {
    pub version: Version,
    pub migrations: &'static [&'static dyn Migration],
}

impl MigrationList {
    pub fn ordered(list: &[Self]) -> bool {
        if list.len() <= 1 {
            return true;
        }
        for i in 1..list.len() {
            if list[i - 1].version >= list[i].version {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(pub u16, pub u16, pub u16);

impl cmp::Ord for Version {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.0.cmp(&other.0) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.1.cmp(&other.1) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.2.cmp(&other.2)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}.{}.{})", self.0, self.1, self.2)
    }
}
