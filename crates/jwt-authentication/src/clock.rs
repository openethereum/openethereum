use chrono::Utc;

/// The clock. Should return the current timestamp.
pub trait Clock {
    /// Returns the number of non-leap seconds since January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
    fn timestamp(&self) -> i64;
}

impl Clock for Utc {
    fn timestamp(&self) -> i64 {
        Utc::now().timestamp()
    }
}
