use std::time::Duration;

pub const TICKS_PER_SECOND: u64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct TickRate {
    per_second: u64,
}

impl Default for TickRate {
    fn default() -> Self {
        Self {
            per_second: TICKS_PER_SECOND,
        }
    }
}

impl TickRate {
    pub fn interval(self) -> Duration {
        Duration::from_micros(1_000_000 / self.per_second)
    }
}
