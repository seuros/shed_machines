#![cfg_attr(not(feature = "std"), no_std)]

//! Adaptive concurrency limits and load shedding.
//!
//! A [`Controller`] watches completed calls and moves a concurrency limit. A
//! [`Codel`] decides whether to admit work based on how long it waited. A
//! [`Budget`] stops a client from amplifying an outage with retries.
//!
//! Nothing here enforces anything. These are the control loops; the caller owns
//! the semaphore, the queue and the dice.

/// Compiles and runs the README's examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub mod aimd;
pub mod budget;
pub mod codel;
pub mod vegas;

pub use aimd::Aimd;
pub use budget::Budget;
pub use codel::{Codel, Verdict};
pub use vegas::Vegas;

/// How a completed call ended, from the limiter's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It finished. The round-trip time is a usable signal.
    Ok,
    /// It timed out, was rejected, or otherwise says the dependency is loaded.
    Overload,
}

/// One completed call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Round-trip time in nanoseconds.
    pub rtt_ns: u64,
    /// Calls in flight when this one started, including itself.
    pub in_flight: u32,
    /// How it ended.
    pub outcome: Outcome,
}

impl Sample {
    #[must_use]
    pub const fn ok(rtt_ns: u64, in_flight: u32) -> Self {
        Self {
            rtt_ns,
            in_flight,
            outcome: Outcome::Ok,
        }
    }

    #[must_use]
    pub const fn overload(rtt_ns: u64, in_flight: u32) -> Self {
        Self {
            rtt_ns,
            in_flight,
            outcome: Outcome::Overload,
        }
    }
}

/// A control loop over a concurrency limit.
pub trait Controller {
    /// Fold one completed call into the limit.
    fn observe(&mut self, sample: Sample);

    /// Calls allowed in flight right now.
    fn limit(&self) -> u32;
}

/// The bounds every controller moves its limit within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Never go below this. A limit of zero would wedge the caller shut.
    pub min: u32,
    /// Never go above this.
    pub max: u32,
}

impl Bounds {
    #[must_use]
    pub const fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    #[must_use]
    pub const fn clamp(&self, limit: u32) -> u32 {
        if limit < self.min {
            self.min
        } else if limit > self.max {
            self.max
        } else {
            limit
        }
    }
}

/// A limit of zero would wedge the caller shut, so one is the lowest floor
/// that still makes progress.
pub const LOWEST_FLOOR: u32 = 1;

/// Arbitrary, and deliberately so: there is no principled default ceiling for
/// an unknown dependency. Set it from what the pool behind it can take.
pub const DEFAULT_CEILING: u32 = 1_000;

impl Default for Bounds {
    fn default() -> Self {
        Self::new(LOWEST_FLOOR, DEFAULT_CEILING)
    }
}

#[cfg(test)]
mod tests;
