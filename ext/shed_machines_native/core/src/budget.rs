//! Client-side adaptive throttling, so retries cannot amplify an outage.

use crate::Refusal;

/// Each [`Budget::decay`] halves both counts.
const DECAY_DIVISOR: u64 = 2;

/// Denominator bias from the formula in Google's SRE book.
const DENOMINATOR_BIAS: f64 = 1.0;

/// Requests each accepted one entitles the client to, from Google's SRE book.
pub const DEFAULT_MULTIPLIER: f64 = 2.0;

/// The smallest multiplier that makes sense: below it a client whose every
/// request is accepted would still be told to shed.
pub const LOWEST_MULTIPLIER: f64 = 1.0;

/// Reports how much outbound work to drop before it leaves the process.
///
/// ```
/// use shed_machines::Budget;
///
/// let mut budget = Budget::new(2.0).ok_or("multiplier")?;
///
/// // Everything is being accepted, so nothing is held back.
/// for _ in 0..100 {
///     budget.requested()?;
///     budget.accepted()?;
/// }
/// assert_eq!(budget.shed_probability(), 0.0);
///
/// // The dependency stops accepting and the client keeps asking.
/// for _ in 0..300 {
///     budget.requested()?;
/// }
/// assert!(budget.shed_probability() > 0.0);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    requests: u64,
    accepts: u64,
    multiplier: f64,
}

impl Budget {
    /// `multiplier` is how many requests each accepted one entitles the client
    /// to. `None` unless it is finite and at least [`LOWEST_MULTIPLIER`]. See
    /// [`DEFAULT_MULTIPLIER`].
    #[must_use]
    pub const fn new(multiplier: f64) -> Option<Self> {
        Self::restored(multiplier, 0, 0)
    }

    /// A budget carrying saved counts: the constructor for a restart. `None`
    /// for a multiplier [`new`](Self::new) would reject. The counts are the
    /// caller's own and any pair is valid: a decay can leave more accepts than
    /// requests.
    #[must_use]
    pub const fn restored(multiplier: f64, requests: u64, accepts: u64) -> Option<Self> {
        if !multiplier.is_finite() || multiplier < LOWEST_MULTIPLIER {
            return None;
        }
        Some(Self {
            requests,
            accepts,
            multiplier,
        })
    }

    /// Count one request sent.
    ///
    /// # Errors
    ///
    /// [`Refusal::Overflow`] when the count is at its maximum.
    pub const fn requested(&mut self) -> Result<(), Refusal> {
        count_one(&mut self.requests)
    }

    /// Count one request the dependency accepted.
    ///
    /// # Errors
    ///
    /// [`Refusal::Overflow`] when the count is at its maximum.
    pub const fn accepted(&mut self) -> Result<(), Refusal> {
        count_one(&mut self.accepts)
    }

    /// Halve both counts. Never refuses.
    pub const fn decay(&mut self) {
        self.requests /= DECAY_DIVISOR;
        self.accepts /= DECAY_DIVISOR;
    }

    /// Requests each accepted one entitles the client to.
    #[must_use]
    pub const fn multiplier(&self) -> f64 {
        self.multiplier
    }

    /// Requests counted.
    #[must_use]
    pub const fn requests(&self) -> u64 {
        self.requests
    }

    /// Accepts counted.
    #[must_use]
    pub const fn accepts(&self) -> u64 {
        self.accepts
    }

    /// The fraction of outbound work to drop locally, in `0.0..1.0`.
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn shed_probability(&self) -> f64 {
        let requests = self.requests as f64;
        let entitled = self.accepts as f64 * self.multiplier;
        let excess = requests - entitled;
        if excess <= 0.0 {
            return 0.0;
        }
        excess / (requests + DENOMINATOR_BIAS)
    }
}

/// One more on `count`, or [`Refusal::Overflow`] with it untouched.
const fn count_one(count: &mut u64) -> Result<(), Refusal> {
    match count.checked_add(1) {
        Some(next) => {
            *count = next;
            Ok(())
        }
        None => Err(Refusal::Overflow),
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            requests: 0,
            accepts: 0,
            multiplier: DEFAULT_MULTIPLIER,
        }
    }
}

#[cfg(test)]
mod tests;
