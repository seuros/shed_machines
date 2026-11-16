//! TCP Vegas, applied to a concurrency limit instead of a window.

use core::time::Duration;

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

/// The hold band: grow below `alpha` queued calls, shrink above `beta`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    alpha: u32,
    beta: u32,
}

impl Band {
    /// `None` unless `alpha <= beta`: a limit told to grow and shrink on the
    /// same estimate has no answer.
    #[must_use]
    pub const fn new(alpha: u32, beta: u32) -> Option<Self> {
        if alpha > beta {
            return None;
        }
        Some(Self { alpha, beta })
    }

    /// Grow while fewer than this many calls look queued.
    #[must_use]
    pub const fn alpha(&self) -> u32 {
        self.alpha
    }

    /// Shrink once more than this many look queued.
    #[must_use]
    pub const fn beta(&self) -> u32 {
        self.beta
    }
}

impl Default for Band {
    fn default() -> Self {
        Self {
            alpha: DEFAULT_ALPHA,
            beta: DEFAULT_BETA,
        }
    }
}

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
/// within two windows when the dependency changes. The span is the
/// `baseline_window` given to [`Vegas::new`].
///
/// ```
/// use core::time::Duration;
/// use shed_machines::vegas::DEFAULT_BASELINE_WINDOW;
/// use shed_machines::{Band, Bounds, Controller, Outcome, Sample, Vegas};
///
/// let bounds = Bounds::new(1, 100).ok_or("bounds")?;
/// let mut control =
///     Vegas::new(bounds, 10, Band::default(), DEFAULT_BASELINE_WINDOW).ok_or("config")?;
///
/// // Establishes the baseline, then grows while the queue estimate stays low.
/// control.observe(Sample::new(Duration::from_millis(1), 10, Outcome::Ok).ok_or("sample")?);
/// assert_eq!(control.limit(), 11);
///
/// // Ten times the baseline latency means the queue is deep.
/// control.observe(Sample::new(Duration::from_millis(10), 11, Outcome::Ok).ok_or("sample")?);
/// assert!(control.limit() < 11);
/// # Ok::<(), &str>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vegas {
    bounds: Bounds,
    limit: u32,
    band: Band,
    /// Samples one baseline window holds, and how many the current one has.
    window: u32,
    seen: u32,
    /// Smallest round-trip time in the window being filled, and in the one
    /// before it. `None` stands for a window with nothing in it.
    current: Option<Duration>,
    previous: Option<Duration>,
}

impl Vegas {
    /// `baseline_window` is how many samples one baseline window spans: a
    /// shorter window follows a dependency whose idle latency drifts, a longer
    /// one holds a steadier baseline when traffic is bursty. `None` when
    /// `limit` is outside `bounds` or `baseline_window` is zero.
    #[must_use]
    pub const fn new(bounds: Bounds, limit: u32, band: Band, baseline_window: u32) -> Option<Self> {
        if !bounds.contains(limit) || baseline_window == 0 {
            return None;
        }
        Some(Self {
            bounds,
            limit,
            band,
            window: baseline_window,
            seen: 0,
            current: None,
            previous: None,
        })
    }

    /// The bounds the limit moves within.
    #[must_use]
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The hold band.
    #[must_use]
    pub const fn band(&self) -> Band {
        self.band
    }

    /// Samples one baseline window spans.
    #[must_use]
    pub const fn baseline_window(&self) -> u32 {
        self.window
    }

    /// The smallest round-trip time in the last window or two, the stand-in for
    /// zero queueing. `None` until a call has completed without overload.
    #[must_use]
    pub fn best_rtt(&self) -> Option<Duration> {
        match (self.current, self.previous) {
            (Some(current), Some(previous)) => Some(current.min(previous)),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        }
    }

    /// Takes `rtt` into the window being filled, retiring that window once it
    /// is full so the baseline cannot outlive the traffic it came from.
    fn rebase(&mut self, rtt: Duration) {
        self.current = Some(self.current.map_or(rtt, |current| current.min(rtt)));
        // `seen < window` between calls, so this cannot overflow.
        self.seen = self.seen.saturating_add(1);
        if self.seen >= self.window {
            self.previous = self.current;
            self.current = None;
            self.seen = 0;
        }
    }

    /// `limit * (rtt - best) / rtt`, the Vegas estimate of how many calls in
    /// flight are waiting rather than working. Kept wide: it is at most the
    /// limit, but it is only ever compared, so it never has to be narrowed.
    fn queued(&self, rtt: Duration) -> u128 {
        let Some(baseline) = self.best_rtt() else {
            return 0;
        };
        let Some(excess) = rtt.checked_sub(baseline) else {
            return 0;
        };
        // Zero excess covers a zero `rtt`, so the divisor below is not zero.
        if excess.is_zero() {
            return 0;
        }
        // A Duration holds under 2^94 nanoseconds; times a u32 that is under
        // 2^126, so the product fits.
        u128::from(self.limit) * excess.as_nanos() / rtt.as_nanos()
    }
}

impl Controller for Vegas {
    fn observe(&mut self, sample: Sample) {
        match sample.outcome() {
            Outcome::Overload => {
                self.limit = self.bounds.clamp(self.limit / OVERLOAD_DIVISOR);
            }
            Outcome::Ok => {
                self.rebase(sample.rtt());
                let queued = self.queued(sample.rtt());
                self.limit = if queued < u128::from(self.band.alpha) {
                    self.bounds.clamp(self.limit.saturating_add(STEP))
                } else if queued > u128::from(self.band.beta) {
                    self.bounds.clamp(self.limit.saturating_sub(STEP))
                } else {
                    self.limit
                };
            }
        }
    }

    fn limit(&self) -> u32 {
        self.limit
    }
}

#[cfg(test)]
mod tests;
