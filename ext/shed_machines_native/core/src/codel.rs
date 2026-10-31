//! Controlled delay: shed by how long work waited, not by how much there is.

/// Acceptable waiting time, from the `CoDel` paper.
pub const DEFAULT_TARGET_NS: u64 = 5_000_000;

/// How long the target may be exceeded before shedding starts, from the paper.
pub const DEFAULT_INTERVAL_NS: u64 = 100_000_000;

/// What to do with a piece of work that has been waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Run it.
    Admit,
    /// Drop it. The queue has been slow for longer than one interval.
    Shed,
}

/// Sheds by how long work waited rather than by how much of it there is.
///
/// Queue length is the wrong signal, because a fast queue can be long and a
/// stalled queue can be short. `CoDel` watches the time work spends waiting and
/// starts shedding only once that stays above target for a full interval, so a
/// burst passes through untouched and a standing queue does not.
///
/// ```
/// use shed_machines::{Codel, Verdict};
///
/// const MS: u64 = 1_000_000;
/// let mut queue = Codel::new(5 * MS, 100 * MS);
///
/// // A burst above target is tolerated until the interval elapses.
/// assert_eq!(queue.admit(20 * MS, 0), Verdict::Admit);
/// assert_eq!(queue.admit(20 * MS, 50 * MS), Verdict::Admit);
/// assert_eq!(queue.admit(20 * MS, 150 * MS), Verdict::Shed);
///
/// // Dropping back under target clears the state.
/// assert_eq!(queue.admit(1 * MS, 200 * MS), Verdict::Admit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Codel {
    target_ns: u64,
    interval_ns: u64,
    above_since_ns: Option<u64>,
    next_shed_at_ns: u64,
    shed_count: u64,
    shedding: bool,
}

impl Codel {
    /// See [`DEFAULT_TARGET_NS`] and [`DEFAULT_INTERVAL_NS`] for the values the
    /// paper settled on.
    #[must_use]
    pub const fn new(target_ns: u64, interval_ns: u64) -> Self {
        Self {
            target_ns,
            interval_ns,
            above_since_ns: None,
            next_shed_at_ns: 0,
            shed_count: 0,
            shedding: false,
        }
    }

    #[must_use]
    pub const fn target_ns(&self) -> u64 {
        self.target_ns
    }

    #[must_use]
    pub const fn interval_ns(&self) -> u64 {
        self.interval_ns
    }

    #[must_use]
    pub const fn shedding(&self) -> bool {
        self.shedding
    }

    /// Successive sheds come faster, as the inverse square root of how many
    /// have happened, so the control law converges instead of oscillating.
    const fn shed_gap_ns(&self) -> u64 {
        let count = if self.shed_count == 0 {
            1
        } else {
            self.shed_count
        };
        let root = count.isqrt();
        self.interval_ns / root
    }

    /// `sojourn_ns` is how long this work waited before being considered.
    pub const fn admit(&mut self, sojourn_ns: u64, now_ns: u64) -> Verdict {
        if sojourn_ns < self.target_ns {
            self.above_since_ns = None;
            self.shedding = false;
            self.shed_count = 0;
            return Verdict::Admit;
        }

        let Some(since) = self.above_since_ns else {
            self.above_since_ns = Some(now_ns);
            return Verdict::Admit;
        };

        if now_ns.saturating_sub(since) < self.interval_ns {
            return Verdict::Admit;
        }

        if self.shedding && now_ns < self.next_shed_at_ns {
            return Verdict::Admit;
        }

        self.shedding = true;
        self.shed_count = self.shed_count.saturating_add(1);
        self.next_shed_at_ns = now_ns.saturating_add(self.shed_gap_ns());
        Verdict::Shed
    }
}

impl Default for Codel {
    fn default() -> Self {
        Self::new(DEFAULT_TARGET_NS, DEFAULT_INTERVAL_NS)
    }
}

#[cfg(test)]
mod tests;
