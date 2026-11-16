#![no_std]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]

//! Adaptive concurrency limits and load shedding.
//!
//! A [`Controller`] watches completed calls and moves a concurrency limit. A
//! [`Codel`] decides whether to admit work based on how long it waited. A
//! [`Budget`] stops a client from amplifying an outage with retries.
//!
//! Nothing here enforces anything. These are the control loops; the caller owns
//! the semaphore, the queue and the dice.
//!
//! # Contract
//!
//! Every value is valid or does not exist: [`Bounds`], [`Sample`],
//! [`Backoff`], [`Band`] and every control loop are built by constructors that
//! answer `None` for a configuration the arithmetic cannot hold. A call that
//! can be malformed answers `Result<_, Refusal>`, and a refusal changes
//! nothing. A decision is not a refusal: [`Verdict::Shed`] is the honoured
//! answer to a well-formed question.

use core::{fmt, time::Duration};

/// Compiles and runs the README's examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub mod aimd;
pub mod budget;
pub mod codel;
pub mod vegas;

pub use aimd::{Aimd, Backoff};
pub use budget::Budget;
pub use codel::{Codel, Verdict};
pub use vegas::{Band, Vegas};

/// Why a call was not honoured. A refused call changed nothing: not a count,
/// not a recorded instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `now` is before the latest instant an honoured call recorded.
    TimeRegression,
    /// A count is at its maximum and cannot take one more.
    Overflow,
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TimeRegression => "now is before the latest honoured instant",
            Self::Overflow => "the count cannot take one more",
        })
    }
}

impl core::error::Error for Refusal {}

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
    rtt: Duration,
    in_flight: u32,
    outcome: Outcome,
}

impl Sample {
    /// `rtt` is the round-trip time, `in_flight` the calls in flight when this
    /// one started, including itself. `None` when `in_flight` is zero: a call
    /// cannot complete without having been in flight.
    #[must_use]
    pub const fn new(rtt: Duration, in_flight: u32, outcome: Outcome) -> Option<Self> {
        if in_flight == 0 {
            return None;
        }
        Some(Self {
            rtt,
            in_flight,
            outcome,
        })
    }

    /// Round-trip time.
    #[must_use]
    pub const fn rtt(&self) -> Duration {
        self.rtt
    }

    /// Calls in flight when this one started, including itself.
    #[must_use]
    pub const fn in_flight(&self) -> u32 {
        self.in_flight
    }

    /// How it ended.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// A control loop over a concurrency limit.
pub trait Controller {
    /// Fold one completed call into the limit. Never refuses: a [`Sample`]
    /// that exists is valid, and the limit saturates inside its [`Bounds`].
    fn observe(&mut self, sample: Sample);

    /// Calls allowed in flight right now.
    fn limit(&self) -> u32;
}

/// The bounds every controller moves its limit within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    min: u32,
    max: u32,
}

impl Bounds {
    /// `None` unless `LOWEST_FLOOR <= min <= max`.
    #[must_use]
    pub const fn new(min: u32, max: u32) -> Option<Self> {
        if min < LOWEST_FLOOR || max < min {
            return None;
        }
        Some(Self { min, max })
    }

    /// Never go below this. A limit of zero would wedge the caller shut.
    #[must_use]
    pub const fn min(&self) -> u32 {
        self.min
    }

    /// Never go above this.
    #[must_use]
    pub const fn max(&self) -> u32 {
        self.max
    }

    /// Whether `limit` is inside the bounds.
    #[must_use]
    pub const fn contains(&self, limit: u32) -> bool {
        self.min <= limit && limit <= self.max
    }

    /// `limit`, pulled inside the bounds.
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
        Self {
            min: LOWEST_FLOOR,
            max: DEFAULT_CEILING,
        }
    }
}

#[cfg(test)]
mod tests;
