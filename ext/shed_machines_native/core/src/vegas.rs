//! TCP Vegas, applied to a concurrency limit instead of a window.

use crate::{Bounds, Controller, Outcome, Sample};

/// Grow while fewer than this many calls look queued. From the Vegas paper,
/// which uses one to three packets; a concurrency limit tolerates the top of
/// that range.
pub const DEFAULT_ALPHA: u32 = 3;

/// Shrink once more than this many look queued. Twice `DEFAULT_ALPHA`, so the
/// hold band is wide enough that a single slow call does not trigger a cut.
pub const DEFAULT_BETA: u32 = 6;

/// Denominator of the fraction kept after an overload. Halving, because an
/// overload says the estimate was wrong rather than slightly high.
pub const OVERLOAD_DIVISOR: u32 = 2;

/// Calls added or removed per sample.
const STEP: u32 = 1;

/// Estimates how many calls are queued rather than working, from the gap
/// between the current round-trip time and the best one ever seen, then steers
/// the limit to keep that estimate inside a band.
///
/// Latency alone cannot distinguish a slow dependency from a deep queue. The
/// ratio can.
///
/// ```
/// use shed_machines::{Bounds, Controller, Sample, Vegas};
///
/// let mut control = Vegas::new(Bounds::new(1, 100), 10);
///
/// // Establishes the baseline, then grows while the queue estimate stays low.
/// control.observe(Sample::ok(1_000_000, 10));
/// assert_eq!(control.limit(), 11);
///
/// // Ten times the baseline latency means the queue is deep.
/// control.observe(Sample::ok(10_000_000, 11));
/// assert!(control.limit() < 11);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vegas {
    bounds: Bounds,
    limit: u32,
    best_rtt_ns: u64,
    alpha: u32,
    beta: u32,
}

impl Vegas {
    #[must_use]
    pub const fn new(bounds: Bounds, limit: u32) -> Self {
        Self {
            bounds,
            limit: bounds.clamp(limit),
            best_rtt_ns: u64::MAX,
            alpha: DEFAULT_ALPHA,
            beta: DEFAULT_BETA,
        }
    }

    /// Grow below `alpha` queued calls, shrink above `beta`, hold between.
    #[must_use]
    pub const fn band(mut self, alpha: u32, beta: u32) -> Self {
        self.alpha = alpha;
        self.beta = beta;
        self
    }

    /// The best round-trip time seen so far, the stand-in for zero queueing.
    #[must_use]
    pub const fn best_rtt_ns(&self) -> Option<u64> {
        if self.best_rtt_ns == u64::MAX {
            None
        } else {
            Some(self.best_rtt_ns)
        }
    }

    /// `limit * (rtt - best) / rtt`, the Vegas estimate of how many calls in
    /// flight are waiting rather than working.
    ///
    /// `excess` never exceeds `rtt_ns`, so the quotient never exceeds the
    /// limit, and nothing here can overflow or need narrowing.
    fn queued(&self, rtt_ns: u64) -> u64 {
        if rtt_ns == 0 || self.best_rtt_ns >= rtt_ns {
            return 0;
        }
        let excess = rtt_ns - self.best_rtt_ns;
        u64::from(self.limit).saturating_mul(excess) / rtt_ns
    }
}

impl Controller for Vegas {
    fn observe(&mut self, sample: Sample) {
        if sample.outcome == Outcome::Overload {
            self.limit = self.bounds.clamp(self.limit / OVERLOAD_DIVISOR);
            return;
        }

        self.best_rtt_ns = self.best_rtt_ns.min(sample.rtt_ns);
        let queued = self.queued(sample.rtt_ns);

        self.limit = if queued < u64::from(self.alpha) {
            self.bounds.clamp(self.limit.saturating_add(STEP))
        } else if queued > u64::from(self.beta) {
            self.bounds.clamp(self.limit.saturating_sub(STEP))
        } else {
            self.limit
        };
    }

    fn limit(&self) -> u32 {
        self.limit
    }
}

#[cfg(test)]
mod tests;
