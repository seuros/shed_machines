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

/// The fraction `kept / of` of the limit that survives an overload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    kept: u32,
    of: u32,
}

impl Backoff {
    /// `None` unless `kept < of`: a fraction that is not below one is not a
    /// decrease, and a zero denominator is not a fraction.
    #[must_use]
    pub const fn new(kept: u32, of: u32) -> Option<Self> {
        if kept >= of {
            return None;
        }
        Some(Self { kept, of })
    }

    /// Numerator.
    #[must_use]
    pub const fn kept(&self) -> u32 {
        self.kept
    }

    /// Denominator.
    #[must_use]
    pub const fn of(&self) -> u32 {
        self.of
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            kept: DEFAULT_BACKOFF_KEPT,
            of: DEFAULT_BACKOFF_OF,
        }
    }
}

/// Grows the limit by one whenever a saturated call succeeds, and cuts it by a
/// factor on any overload. The limit only grows while the caller is actually
/// using it, so an idle client does not drift upward.
///
/// ```
/// use core::time::Duration;
/// use shed_machines::{Aimd, Backoff, Bounds, Controller, Outcome, Sample};
///
/// let bounds = Bounds::new(1, 100).ok_or("bounds")?;
/// let mut control = Aimd::new(bounds, 10, Backoff::default()).ok_or("limit")?;
/// let rtt = Duration::from_micros(1);
///
/// control.observe(Sample::new(rtt, 10, Outcome::Ok).ok_or("sample")?);
/// assert_eq!(control.limit(), 11);
///
/// control.observe(Sample::new(rtt, 11, Outcome::Overload).ok_or("sample")?);
/// assert_eq!(control.limit(), 8);
/// # Ok::<(), &str>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aimd {
    bounds: Bounds,
    limit: u32,
    backoff: Backoff,
}

impl Aimd {
    /// `None` when `limit` is outside `bounds`.
    #[must_use]
    pub const fn new(bounds: Bounds, limit: u32, backoff: Backoff) -> Option<Self> {
        if !bounds.contains(limit) {
            return None;
        }
        Some(Self {
            bounds,
            limit,
            backoff,
        })
    }

    /// The bounds the limit moves within.
    #[must_use]
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The fraction kept after an overload.
    #[must_use]
    pub const fn backoff(&self) -> Backoff {
        self.backoff
    }

    /// The product is widened so the quotient is exact. `kept < of`, so the
    /// quotient is below the limit and always fits back into a `u32`.
    fn kept_limit(&self) -> u32 {
        let scaled =
            u64::from(self.limit) * u64::from(self.backoff.kept) / u64::from(self.backoff.of);
        u32::try_from(scaled).unwrap_or_else(|_| {
            debug_assert!(
                false,
                "kept < of keeps the backed-off limit below the limit"
            );
            // The most conservative answer to an overload: the floor.
            self.bounds.min
        })
    }
}

impl Controller for Aimd {
    fn observe(&mut self, sample: Sample) {
        self.limit = match sample.outcome() {
            Outcome::Overload => self.bounds.clamp(self.kept_limit()),
            Outcome::Ok if sample.in_flight() >= self.limit => {
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
