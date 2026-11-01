//! TCP Vegas, applied to a concurrency limit instead of a window.

use crate::{Bounds, Controller, Outcome, Sample};

/// Grow while fewer than this many calls look queued. From the Vegas paper,
/// which uses one to three packets; a concurrency limit tolerates the top of
/// that range.
pub const DEFAULT_ALPHA: u32 = 3;

/// Shrink once more than this many look queued. Twice `DEFAULT_ALPHA`, so the
/// hold band is wide enough that a single slow call does not trigger a cut.
pub const DEFAULT_BETA: u32 = 6;

/// Denominator of the fraction kept after an overload.
pub const OVERLOAD_DIVISOR: u32 = 2;

/// Samples one baseline window spans. The baseline is the smallest round-trip
/// time in the current window or the one before it, so it always reflects
/// between this many recent calls and twice as many.
pub const DEFAULT_BASELINE_WINDOW: u32 = 1_000;

/// Calls added or removed per sample.
const STEP: u32 = 1;

/// Estimates how many calls are queued rather than working, from the gap
/// between the current round-trip time and the smallest one seen recently, then
/// steers the limit to keep that estimate inside a band.
///
/// # The work has to be homogeneous
///
/// The queue estimate reads every nanosecond above the baseline as time spent
/// waiting, which only holds when any two calls would take about as long on an
/// idle dependency. Over a mix whose service times differ by orders of
/// magnitude the baseline settles on the fastest kind of work, every slower
/// call looks like a deep queue, and the limit sinks to [`Bounds`]' floor and
/// stays there — no amount of tuning `alpha` and `beta` fixes it, because the
/// signal itself is measuring the wrong thing. Give heterogeneous work
/// [`Aimd`](crate::Aimd) instead: it steers on outcomes rather than latency, so
/// a slow call is not mistaken for a queued one. One `Vegas` per class of work
/// is the other way out.
///
/// # The baseline is windowed
///
/// A baseline that only ever shrank would pin itself to the luckiest call the
/// process ever made and never recover, so one unusually fast sample, or a
/// dependency that has since legitimately slowed down, would hold the limit at
/// the floor for good. The baseline is instead the smallest round-trip time in
/// the current window of [`DEFAULT_BASELINE_WINDOW`] samples or in the one
/// before it, which keeps it stable while traffic is steady and lets it rise
/// within two windows when the dependency changes. Set the span with
/// [`baseline_window`](Self::baseline_window).
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
    /// Smallest round-trip time in the window being filled, and in the one
    /// before it. [`u64::MAX`] stands for a window with nothing in it.
    current_ns: u64,
    previous_ns: u64,
    /// Samples into the current window, and how many it holds.
    seen: u32,
    window: u32,
    alpha: u32,
    beta: u32,
}

impl Vegas {
    #[must_use]
    pub const fn new(bounds: Bounds, limit: u32) -> Self {
        Self {
            bounds,
            limit: bounds.clamp(limit),
            current_ns: u64::MAX,
            previous_ns: u64::MAX,
            seen: 0,
            window: DEFAULT_BASELINE_WINDOW,
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

    /// Samples one baseline window spans, at least one. A shorter window
    /// follows a dependency whose idle latency drifts; a longer one holds a
    /// steadier baseline when traffic is bursty.
    #[must_use]
    pub const fn baseline_window(mut self, samples: u32) -> Self {
        self.window = samples;
        self
    }

    /// The smallest round-trip time in the last window or two, the stand-in for
    /// zero queueing.
    #[must_use]
    pub const fn best_rtt_ns(&self) -> Option<u64> {
        let baseline = self.baseline();
        if baseline == u64::MAX {
            None
        } else {
            Some(baseline)
        }
    }

    const fn baseline(&self) -> u64 {
        if self.current_ns < self.previous_ns {
            self.current_ns
        } else {
            self.previous_ns
        }
    }

    /// Takes `rtt_ns` into the window being filled, retiring that window once
    /// it is full so the baseline cannot outlive the traffic it came from.
    const fn rebase(&mut self, rtt_ns: u64) {
        if rtt_ns < self.current_ns {
            self.current_ns = rtt_ns;
        }
        self.seen += 1;
        if self.seen >= self.window {
            self.previous_ns = self.current_ns;
            self.current_ns = u64::MAX;
            self.seen = 0;
        }
    }

    /// `limit * (rtt - best) / rtt`, the Vegas estimate of how many calls in
    /// flight are waiting rather than working.
    fn queued(&self, rtt_ns: u64) -> u64 {
        let baseline = self.baseline();
        if rtt_ns == 0 || baseline >= rtt_ns {
            return 0;
        }
        let excess = rtt_ns - baseline;
        let scaled = u128::from(self.limit) * u128::from(excess) / u128::from(rtt_ns);
        u64::try_from(scaled).unwrap_or(u64::MAX)
    }
}

impl Controller for Vegas {
    fn observe(&mut self, sample: Sample) {
        if sample.outcome == Outcome::Overload {
            self.limit = self.bounds.clamp(self.limit / OVERLOAD_DIVISOR);
            return;
        }

        self.rebase(sample.rtt_ns);
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
