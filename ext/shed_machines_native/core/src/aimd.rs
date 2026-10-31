//! Additive increase, multiplicative decrease.

use crate::{Bounds, Controller, Outcome, Sample};

/// Numerator of the fraction kept after an overload.
pub const DEFAULT_BACKOFF_KEPT: u32 = 3;

/// Denominator of the fraction kept after an overload. Three quarters backs
/// off faster than a dependency usually recovers, without the halving that
/// makes a limit oscillate.
pub const DEFAULT_BACKOFF_OF: u32 = 4;

/// Calls added per saturated success. One is what makes this additive.
const INCREASE: u32 = 1;

/// Grows the limit by one whenever a saturated call succeeds, and cuts it by a
/// factor on any overload. The limit only grows while the caller is actually
/// using it, so an idle client does not drift upward.
///
/// ```
/// use shed_machines::{Aimd, Bounds, Controller, Sample};
///
/// let mut control = Aimd::new(Bounds::new(1, 100), 10);
///
/// control.observe(Sample::ok(1_000, 10));
/// assert_eq!(control.limit(), 11);
///
/// control.observe(Sample::overload(1_000, 11));
/// assert_eq!(control.limit(), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aimd {
    bounds: Bounds,
    limit: u32,
    kept: u32,
    of: u32,
}

impl Aimd {
    #[must_use]
    pub const fn new(bounds: Bounds, limit: u32) -> Self {
        Self {
            bounds,
            limit: bounds.clamp(limit),
            kept: DEFAULT_BACKOFF_KEPT,
            of: DEFAULT_BACKOFF_OF,
        }
    }

    /// Keep `kept / of` of the limit after an overload. See
    /// [`DEFAULT_BACKOFF_KEPT`].
    #[must_use]
    pub const fn backoff(mut self, kept: u32, of: u32) -> Self {
        self.kept = kept;
        self.of = of;
        self
    }
}

impl Aimd {
    /// A zero denominator keeps the limit rather than dividing by zero, and the
    /// product is widened so a representable quotient is not lost to saturation.
    fn kept_limit(&self) -> u32 {
        if self.of == 0 {
            return self.limit;
        }
        let scaled = u64::from(self.limit) * u64::from(self.kept) / u64::from(self.of);
        u32::try_from(scaled).unwrap_or(u32::MAX)
    }
}

impl Controller for Aimd {
    fn observe(&mut self, sample: Sample) {
        self.limit = match sample.outcome {
            Outcome::Overload => self.bounds.clamp(self.kept_limit()),
            Outcome::Ok if sample.in_flight >= self.limit => {
                self.bounds.clamp(self.limit.saturating_add(INCREASE))
            }
            Outcome::Ok => self.limit,
        };
    }

    fn limit(&self) -> u32 {
        self.limit
    }
}

#[cfg(test)]
mod tests;
