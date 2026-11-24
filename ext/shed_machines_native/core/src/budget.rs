//! Client-side adaptive throttling, so retries cannot amplify an outage.

use core::time::Duration;

use crate::Refusal;

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
        self.decay_by(1);
    }

    const fn decay_by(&mut self, intervals: u32) {
        if intervals >= u64::BITS {
            self.requests = 0;
            self.accepts = 0;
        } else {
            self.requests >>= intervals;
            self.accepts >>= intervals;
        }
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

/// Retry budget with whole-interval decay and a minimum request sample.
///
/// Time is monotonic elapsed time from a caller-selected epoch. Only successful
/// records advance the observed clock; queries project the budget without
/// changing it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedBudget {
    budget: Budget,
    interval: Duration,
    min_requests: u64,
    boundary: Duration,
    observed: Duration,
}

impl TimedBudget {
    /// Start a decay interval at `now` with the supplied budget. `None` for
    /// a zero interval. A `min_requests` of zero disables the sample floor.
    #[must_use]
    pub const fn new(
        budget: Budget,
        now: Duration,
        interval: Duration,
        min_requests: u64,
    ) -> Option<Self> {
        if interval.is_zero() {
            return None;
        }
        Some(Self {
            budget,
            interval,
            min_requests,
            boundary: now,
            observed: now,
        })
    }

    /// Decay if due, then count a request sent.
    ///
    /// # Errors
    ///
    /// [`Refusal::TimeRegression`] before the last recorded instant, or
    /// [`Refusal::Overflow`] if the aged request count cannot take one more.
    pub fn requested(&mut self, now: Duration) -> Result<(), Refusal> {
        self.record(now, Budget::requested)
    }

    /// Decay if due, then count a request accepted.
    ///
    /// # Errors
    ///
    /// [`Refusal::TimeRegression`] before the last recorded instant, or
    /// [`Refusal::Overflow`] if the aged accept count cannot take one more.
    pub fn accepted(&mut self, now: Duration) -> Result<(), Refusal> {
        self.record(now, Budget::accepted)
    }

    /// The aged counts and multiplier at `now`, without changing this budget.
    ///
    /// # Errors
    ///
    /// [`Refusal::TimeRegression`] before the last recorded instant.
    pub fn budget_at(&self, now: Duration) -> Result<Budget, Refusal> {
        self.aged(now).map(|(budget, _)| budget)
    }

    /// The aged fraction of outbound work to drop, or zero below the sample floor.
    ///
    /// # Errors
    ///
    /// [`Refusal::TimeRegression`] before the last recorded instant.
    pub fn shed_probability(&self, now: Duration) -> Result<f64, Refusal> {
        let budget = self.budget_at(now)?;
        Ok(if budget.requests() < self.min_requests {
            0.0
        } else {
            budget.shed_probability()
        })
    }

    fn record(
        &mut self,
        now: Duration,
        add: impl FnOnce(&mut Budget) -> Result<(), Refusal>,
    ) -> Result<(), Refusal> {
        let (mut budget, boundary) = self.aged(now)?;
        add(&mut budget)?;
        self.budget = budget;
        self.boundary = boundary;
        self.observed = now;
        Ok(())
    }

    fn aged(&self, now: Duration) -> Result<(Budget, Duration), Refusal> {
        if now < self.observed {
            return Err(Refusal::TimeRegression);
        }

        let elapsed = now.saturating_sub(self.boundary).as_nanos();
        let interval = self.interval.as_nanos();
        if elapsed < interval {
            return Ok((self.budget, self.boundary));
        }
        let mut budget = self.budget;
        budget.decay_by(u32::try_from(elapsed / interval).unwrap_or(u32::MAX));
        let remainder = elapsed % interval;
        // The remainder is no larger than `elapsed`, itself a Duration.
        let seconds = u64::try_from(remainder / 1_000_000_000).unwrap_or(u64::MAX);
        let nanos = u32::try_from(remainder % 1_000_000_000).unwrap_or_default();
        Ok((budget, now.saturating_sub(Duration::new(seconds, nanos))))
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
