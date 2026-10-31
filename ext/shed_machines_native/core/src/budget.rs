//! Client-side adaptive throttling, so retries cannot amplify an outage.

/// Each [`Budget::decay`] halves both counts.
const DECAY_DIVISOR: u64 = 2;

/// Added to the denominator so a client with no history sheds nothing instead
/// of dividing by zero. From the formula in Google's SRE book.
const DENOMINATOR_BIAS: f64 = 1.0;

/// Requests each accepted one entitles the client to, from Google's SRE book.
pub const DEFAULT_MULTIPLIER: f64 = 2.0;

/// Reports how much outbound work to drop before it leaves the process.
///
/// A client that keeps retrying a failing dependency multiplies the load that
/// broke it. This tracks what the client asked for against what was accepted,
/// and holds the next wave back in proportion.
///
/// The caller owns the dice: [`Budget::shed_probability`] is a fraction, not a
/// decision, so the core needs no random number generator.
///
/// ```
/// use shed_machines::Budget;
///
/// let mut budget = Budget::new(2.0);
///
/// // Everything is being accepted, so nothing is held back.
/// for _ in 0..100 {
///     budget.requested();
///     budget.accepted();
/// }
/// assert_eq!(budget.shed_probability(), 0.0);
///
/// // The dependency stops accepting and the client keeps asking.
/// for _ in 0..300 {
///     budget.requested();
/// }
/// assert!(budget.shed_probability() > 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    requests: u64,
    accepts: u64,
    multiplier: f64,
}

impl Budget {
    /// `multiplier` is how many requests each accepted one entitles the client
    /// to. See [`DEFAULT_MULTIPLIER`].
    #[must_use]
    pub const fn new(multiplier: f64) -> Self {
        Self {
            requests: 0,
            accepts: 0,
            multiplier,
        }
    }

    pub const fn requested(&mut self) {
        self.requests = self.requests.saturating_add(1);
    }

    pub const fn accepted(&mut self) {
        self.accepts = self.accepts.saturating_add(1);
    }

    /// Drop both counts to a fraction of themselves, so the budget forgets an
    /// outage instead of holding it against the dependency forever.
    pub const fn decay(&mut self) {
        self.requests /= DECAY_DIVISOR;
        self.accepts /= DECAY_DIVISOR;
    }

    #[must_use]
    pub const fn multiplier(&self) -> f64 {
        self.multiplier
    }

    #[must_use]
    pub const fn requests(&self) -> u64 {
        self.requests
    }

    #[must_use]
    pub const fn accepts(&self) -> u64 {
        self.accepts
    }

    /// The fraction of outbound work to drop locally, in `0.0..1.0`.
    #[must_use]
    pub fn shed_probability(&self) -> f64 {
        let requests = u32::try_from(self.requests.min(u64::from(u32::MAX))).unwrap_or(u32::MAX);
        let accepts = u32::try_from(self.accepts.min(u64::from(u32::MAX))).unwrap_or(u32::MAX);
        let entitled = f64::from(accepts) * self.multiplier;
        let excess = f64::from(requests) - entitled;
        if excess <= 0.0 {
            return 0.0;
        }
        excess / (f64::from(requests) + DENOMINATOR_BIAS)
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(DEFAULT_MULTIPLIER)
    }
}

#[cfg(test)]
mod tests;
