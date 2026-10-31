use crate::{Aimd, Bounds, Controller, Sample};

#[test]
fn the_limit_grows_only_while_it_is_saturated() {
    let mut control = Aimd::new(Bounds::new(1, 100), 10);

    control.observe(Sample::ok(1_000, 3));
    assert_eq!(control.limit(), 10);

    control.observe(Sample::ok(1_000, 10));
    assert_eq!(control.limit(), 11);
}

#[test]
fn an_overload_cuts_the_limit_by_the_backoff() {
    let mut control = Aimd::new(Bounds::new(1, 100), 20).backoff(1, 2);
    control.observe(Sample::overload(1_000, 20));

    assert_eq!(control.limit(), 10);
}

#[test]
fn repeated_overload_stops_at_the_floor_instead_of_zero() {
    let mut control = Aimd::new(Bounds::new(4, 100), 100).backoff(1, 2);
    for _ in 0..20 {
        control.observe(Sample::overload(1_000, 1));
    }

    assert_eq!(control.limit(), 4);
}

#[test]
fn the_limit_never_passes_the_ceiling() {
    let mut control = Aimd::new(Bounds::new(1, 12), 10);
    for _ in 0..50 {
        control.observe(Sample::ok(1_000, u32::MAX));
    }

    assert_eq!(control.limit(), 12);
}
