use matryona_machines::testing::XorShift64;

use crate::tests::{bounds, ok, overload};
use crate::{Aimd, Backoff, Controller};

fn control(min: u32, max: u32, limit: u32, backoff: Backoff) -> Aimd {
    Aimd::new(bounds(min, max), limit, backoff).expect("a limit inside the bounds")
}

fn halve() -> Backoff {
    Backoff::new(1, 2).expect("a half is a fraction")
}

#[test]
fn the_limit_grows_only_while_it_is_saturated() {
    let mut control = control(1, 100, 10, Backoff::default());

    control.observe(ok(1_000, 3));
    assert_eq!(control.limit(), 10);

    control.observe(ok(1_000, 10));
    assert_eq!(control.limit(), 11);
}

#[test]
fn an_overload_cuts_the_limit_by_the_backoff() {
    let mut control = control(1, 100, 20, halve());
    control.observe(overload(1_000, 20));

    assert_eq!(control.limit(), 10);
    assert_eq!(control.backoff(), halve());
    assert_eq!(control.bounds(), bounds(1, 100));
}

#[test]
fn the_limit_stops_at_the_floor_and_the_ceiling() {
    let mut control = control(4, 12, 10, halve());
    for _ in 0..20 {
        control.observe(overload(1_000, 1));
    }
    assert_eq!(control.limit(), 4);

    for _ in 0..50 {
        control.observe(ok(1_000, u32::MAX));
    }
    assert_eq!(control.limit(), 12);
}

#[test]
fn the_widest_limit_backs_off_without_overflowing() {
    let mut control = control(1, u32::MAX, u32::MAX, Backoff::default());
    control.observe(ok(1, u32::MAX));
    assert_eq!(control.limit(), u32::MAX);

    control.observe(overload(1, u32::MAX));
    assert_eq!(u64::from(control.limit()), u64::from(u32::MAX) * 3 / 4);
}

#[test]
fn a_configuration_that_is_not_a_decrease_inside_the_bounds_does_not_exist() {
    assert_eq!(Backoff::new(1, 0), None);
    assert_eq!(Backoff::new(0, 0), None);
    assert_eq!(Backoff::new(4, 4), None);
    assert_eq!(Backoff::new(5, 4), None);
    assert_eq!(Backoff::new(3, 4), Some(Backoff::default()));
    assert_eq!(
        Backoff::new(0, 1).map(|backoff| (backoff.kept(), backoff.of())),
        Some((0, 1))
    );

    assert_eq!(Aimd::new(bounds(2, 10), 1, Backoff::default()), None);
    assert_eq!(Aimd::new(bounds(2, 10), 11, Backoff::default()), None);
    assert!(Aimd::new(bounds(2, 10), 2, Backoff::default()).is_some());
    assert!(Aimd::new(bounds(2, 10), 10, Backoff::default()).is_some());
}

/// AIMD restated in wide integers: an overload keeps `floor(limit × kept /
/// of)`, a saturated success adds one, and the bounds clamp both.
struct Model {
    min: u128,
    max: u128,
    limit: u128,
    kept: u128,
    of: u128,
}

impl Model {
    fn observe(&mut self, overloaded: bool, in_flight: u128) {
        let next = if overloaded {
            self.limit * self.kept / self.of
        } else if in_flight >= self.limit {
            self.limit + 1
        } else {
            self.limit
        };
        self.limit = next.clamp(self.min, self.max);
    }
}

/// A draw from `low..=high`, skewed toward both ends.
fn pick(dice: &mut XorShift64, low: u32, high: u32) -> u32 {
    let span = u64::from(high - low) + 1;
    match dice.next_u64() % 8 {
        0 => low,
        1 => high,
        _ => low + u32::try_from(dice.next_u64() % span).expect("below the span"),
    }
}

#[test]
fn the_limit_follows_a_reference_model() {
    let mut dice = XorShift64::new(4_017_093_231);
    for _ in 0..300 {
        // Narrow bounds most of the time, the full range now and then.
        let (min, max) = if dice.next_u64().is_multiple_of(5) {
            (1, u32::MAX)
        } else {
            let min = pick(&mut dice, 1, 40);
            (min, min + pick(&mut dice, 0, 60))
        };
        let limit = pick(&mut dice, min, max);
        let of = pick(&mut dice, 1, 9);
        let kept = pick(&mut dice, 0, of - 1);
        let backoff = Backoff::new(kept, of).expect("kept < of");
        let mut control = control(min, max, limit, backoff);
        let mut model = Model {
            min: u128::from(min),
            max: u128::from(max),
            limit: u128::from(limit),
            kept: u128::from(kept),
            of: u128::from(of),
        };
        for _ in 0..200 {
            let overloaded = dice.next_u64().is_multiple_of(4);
            // Around the current limit, so saturation goes both ways.
            let in_flight = pick(&mut dice, 1, control.limit().saturating_add(2));
            let rtt = dice.next_u64() % 1_000_000;
            let sample = if overloaded {
                overload(rtt, in_flight)
            } else {
                ok(rtt, in_flight)
            };
            control.observe(sample);
            model.observe(overloaded, u128::from(in_flight));
            assert_eq!(u128::from(control.limit()), model.limit);
            assert_eq!(control.backoff(), backoff);
        }
    }
}
