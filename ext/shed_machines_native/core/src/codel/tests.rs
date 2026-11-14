use core::time::Duration;

use crate::codel::{DEFAULT_INTERVAL, DEFAULT_TARGET};
use crate::tests::Dice;
use crate::{Codel, Refusal, Verdict};

const fn ms(count: u64) -> Duration {
    Duration::from_millis(count)
}

fn queue() -> Codel {
    Codel::new(ms(5), ms(100)).expect("5 ms over 100 ms is a configuration")
}

/// A burst shorter than the interval passes; a queue standing past it sheds;
/// dropping back under target clears the run.
#[test]
fn a_standing_queue_sheds_once_the_interval_elapses_until_it_drains() {
    let mut codel = queue();
    assert_eq!(codel.admit(ms(50), ms(0)), Ok(Verdict::Admit));
    assert_eq!(codel.admit(ms(50), ms(99)), Ok(Verdict::Admit));
    assert!(!codel.shedding());

    assert_eq!(codel.admit(ms(50), ms(100)), Ok(Verdict::Shed));
    assert!(codel.shedding());
    assert_eq!(codel.shed_count(), 1);

    assert_eq!(codel.admit(ms(1), ms(101)), Ok(Verdict::Admit));

    assert!(!codel.shedding());
    assert_eq!(codel.shed_count(), 0);
}

#[test]
fn sheds_come_closer_together_the_longer_the_queue_stands() {
    const GAPS: usize = 4;

    let mut codel = queue();
    assert_eq!(codel.admit(ms(50), ms(0)), Ok(Verdict::Admit));

    let mut gaps = [Duration::ZERO; GAPS];
    let mut found = 0;
    let mut last_shed_at = None;
    let mut now = ms(100);
    while found < GAPS {
        if codel.admit(ms(50), now) == Ok(Verdict::Shed) {
            if let Some(last) = last_shed_at {
                gaps[found] = now.saturating_sub(last);
                found += 1;
            }
            last_shed_at = Some(now);
        }
        now += ms(1);
    }

    assert!(
        gaps.windows(2).all(|pair| pair[1] <= pair[0]),
        "gaps widened: {gaps:?}"
    );
}

#[test]
fn a_configuration_with_a_zero_span_does_not_exist() {
    assert_eq!(Codel::new(Duration::ZERO, ms(100)), None);
    assert_eq!(Codel::new(ms(5), Duration::ZERO), None);
    assert_eq!(
        Codel::new(DEFAULT_TARGET, DEFAULT_INTERVAL),
        Some(Codel::default())
    );

    let codel = queue();
    assert_eq!((codel.target(), codel.interval()), (ms(5), ms(100)));
}

#[test]
fn an_earlier_instant_is_refused_and_changes_nothing() {
    let mut codel = queue();
    assert_eq!(codel.admit(ms(50), ms(10)), Ok(Verdict::Admit));
    assert_eq!(codel.admit(ms(50), ms(110)), Ok(Verdict::Shed));
    let before = codel;

    assert_eq!(codel.admit(ms(50), ms(109)), Err(Refusal::TimeRegression));
    assert_eq!(codel, before);
    // An answer under target would have reset the run: refused, it did not.
    assert_eq!(codel.admit(ms(1), ms(109)), Err(Refusal::TimeRegression));
    assert_eq!(codel, before);
    assert!(codel.shedding());
}

/// Work under target is admitted whatever the time, so it records no instant:
/// a later answer above target may still use an instant before it.
#[test]
fn an_answer_that_does_not_depend_on_the_time_records_none() {
    let mut codel = queue();
    assert_eq!(codel.admit(ms(50), ms(10)), Ok(Verdict::Admit));
    assert_eq!(codel.admit(ms(1), ms(500)), Ok(Verdict::Admit));

    assert_eq!(codel.admit(ms(50), ms(20)), Ok(Verdict::Admit));
    assert_eq!(codel.admit(ms(50), ms(19)), Err(Refusal::TimeRegression));
}

/// The control law written out plainly, in nanoseconds.
#[derive(Default)]
struct Model {
    latest: u128,
    above_since: Option<u128>,
    next_shed_at: u128,
    sheds: u64,
    shedding: bool,
}

impl Model {
    const TARGET: u128 = 5_000_000;
    const INTERVAL: u128 = 100_000_000;

    fn admit(&mut self, sojourn: u128, now: u128) -> Result<Verdict, Refusal> {
        if now < self.latest {
            return Err(Refusal::TimeRegression);
        }
        if sojourn < Self::TARGET {
            *self = Self {
                latest: self.latest,
                ..Self::default()
            };
            return Ok(Verdict::Admit);
        }
        self.latest = now;
        let Some(since) = self.above_since else {
            self.above_since = Some(now);
            return Ok(Verdict::Admit);
        };
        if now - since < Self::INTERVAL || (self.shedding && now < self.next_shed_at) {
            return Ok(Verdict::Admit);
        }
        self.shedding = true;
        self.sheds += 1;
        self.next_shed_at = now + Self::INTERVAL / u128::from(self.sheds.isqrt());
        Ok(Verdict::Shed)
    }
}

#[test]
fn the_queue_follows_a_reference_model() {
    let mut dice = Dice(5_150_115_101);
    for _ in 0..200 {
        let mut codel = queue();
        let mut model = Model::default();
        let mut now: u64 = 0;
        for _ in 0..500 {
            let roll = dice.roll();
            // Mostly forward, sometimes back, so regressions are exercised.
            now = if roll.is_multiple_of(8) {
                now.saturating_sub(roll % 30_000_000)
            } else {
                now + roll % 40_000_000
            };
            let sojourn = (roll >> 20) % 12_000_000;
            let before = codel;

            let answer = codel.admit(Duration::from_nanos(sojourn), Duration::from_nanos(now));

            assert_eq!(answer, model.admit(u128::from(sojourn), u128::from(now)));
            if answer.is_err() {
                assert_eq!(codel, before);
            }
            assert_eq!(codel.shedding(), model.shedding);
            assert_eq!(codel.shed_count(), model.sheds);
        }
    }
}
