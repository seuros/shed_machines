//! Controlled delay: shed by how long work waited, not by how much there is.

use core::num::{NonZeroU32, NonZeroU64};
use core::time::Duration;

use crate::Refusal;

/// Acceptable waiting time, from the `CoDel` paper.
pub const DEFAULT_TARGET: Duration = Duration::from_millis(5);

/// How long the target may be exceeded before shedding starts, from the paper.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(100);

/// What to do with a piece of work that has been waiting. Both are honoured
/// answers: the question was well formed and this is the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Run it.
    Admit,
    /// Drop it. The queue has been slow for longer than one interval.
    Shed,
}

/// Sheds by how long work waited rather than by how much of it there is.
///
/// The next-shed instant saturates at `Duration::MAX` and the shed counter at
/// `u64::MAX`; neither saturation is a refusal.
///
/// # Shedding is a decision, not a refusal
///
/// "Admit this work, which waited this long?" is a well-formed question, and
/// [`Verdict::Shed`] is the controller's answer to it. The call was honoured:
/// it recorded the instant, counted the shed and scheduled the next one,
/// because that bookkeeping is the control law's state. A [`Refusal`] is kept
/// for a call the controller cannot answer at all, and changes nothing.
///
/// # Which answers record the clock
///
/// Work under target is admitted whatever the time, so that answer records no
/// instant. Every answer for work at or above target depends on `now` and
/// records it; a later `now` before it is [`Refusal::TimeRegression`].
///
/// ```
/// use core::time::Duration;
/// use shed_machines::{Codel, Verdict};
///
/// const fn ms(count: u64) -> Duration {
///     Duration::from_millis(count)
/// }
/// let mut queue = Codel::new(ms(5), ms(100)).ok_or("config")?;
///
/// // A burst above target is tolerated until the interval elapses.
/// assert_eq!(queue.admit(ms(20), ms(0)), Ok(Verdict::Admit));
/// assert_eq!(queue.admit(ms(20), ms(50)), Ok(Verdict::Admit));
/// assert_eq!(queue.admit(ms(20), ms(150)), Ok(Verdict::Shed));
///
/// // Dropping back under target clears the state.
/// assert_eq!(queue.admit(ms(1), ms(200)), Ok(Verdict::Admit));
/// # Ok::<(), &str>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Codel {
    target: Duration,
    interval: Duration,
    /// The latest instant an honoured call recorded.
    now: Duration,
    above_since: Option<Duration>,
    next_shed_at: Duration,
    shed_count: u64,
    shedding: bool,
}

impl Codel {
    /// `None` when `target` or `interval` is zero. See [`DEFAULT_TARGET`] and
    /// [`DEFAULT_INTERVAL`] for the values the paper settled on.
    #[must_use]
    pub const fn new(target: Duration, interval: Duration) -> Option<Self> {
        if target.is_zero() || interval.is_zero() {
            return None;
        }
        Some(Self::with(target, interval))
    }

    const fn with(target: Duration, interval: Duration) -> Self {
        Self {
            target,
            interval,
            now: Duration::ZERO,
            above_since: None,
            next_shed_at: Duration::ZERO,
            shed_count: 0,
            shedding: false,
        }
    }

    /// The acceptable waiting time.
    #[must_use]
    pub const fn target(&self) -> Duration {
        self.target
    }

    /// How long the target may be exceeded before shedding starts.
    #[must_use]
    pub const fn interval(&self) -> Duration {
        self.interval
    }

    /// Whether the last answer for work above target was part of a shedding run.
    #[must_use]
    pub const fn shedding(&self) -> bool {
        self.shedding
    }

    /// Sheds since the queue last dropped under target.
    #[must_use]
    pub const fn shed_count(&self) -> u64 {
        self.shed_count
    }

    /// Successive sheds come faster, as the inverse square root of how many
    /// have happened.
    fn shed_gap(&self) -> Duration {
        let root = NonZeroU64::new(self.shed_count).map_or(NonZeroU64::MIN, NonZeroU64::isqrt);
        let root = NonZeroU32::try_from(root).unwrap_or_else(|_| {
            debug_assert!(false, "the square root of a u64 fits in a u32");
            // The exact root of the largest count, and the shortest gap.
            NonZeroU32::MAX
        });
        self.interval / root.get()
    }

    /// `sojourn` is how long this work waited before being considered, `now`
    /// the current instant.
    ///
    /// # Errors
    ///
    /// [`Refusal::TimeRegression`] when `now` is before the latest instant an
    /// honoured call recorded. Nothing changed.
    pub fn admit(&mut self, sojourn: Duration, now: Duration) -> Result<Verdict, Refusal> {
        if now < self.now {
            return Err(Refusal::TimeRegression);
        }
        if sojourn < self.target {
            self.above_since = None;
            self.shedding = false;
            self.shed_count = 0;
            return Ok(Verdict::Admit);
        }
        self.now = now;

        let Some(since) = self.above_since else {
            self.above_since = Some(now);
            return Ok(Verdict::Admit);
        };
        if now.saturating_sub(since) < self.interval {
            return Ok(Verdict::Admit);
        }
        if self.shedding && now < self.next_shed_at {
            return Ok(Verdict::Admit);
        }

        self.shedding = true;
        self.shed_count = self.shed_count.saturating_add(1);
        self.next_shed_at = now.saturating_add(self.shed_gap());
        Ok(Verdict::Shed)
    }
}

impl Default for Codel {
    fn default() -> Self {
        Self::with(DEFAULT_TARGET, DEFAULT_INTERVAL)
    }
}

#[cfg(test)]
mod tests;
