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
