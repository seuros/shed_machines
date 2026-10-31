use crate::{Bounds, Controller, Sample, Vegas};

#[test]
fn the_first_sample_sets_the_baseline() {
    let mut control = Vegas::new(Bounds::new(1, 100), 10);
    assert_eq!(control.best_rtt_ns(), None);

    control.observe(Sample::ok(5_000_000, 10));

    assert_eq!(control.best_rtt_ns(), Some(5_000_000));
}

#[test]
fn a_latency_at_the_baseline_reads_as_an_empty_queue_and_grows() {
    let mut control = Vegas::new(Bounds::new(1, 100), 10);
    control.observe(Sample::ok(1_000_000, 10));
    control.observe(Sample::ok(1_000_000, 11));

    assert_eq!(control.limit(), 12);
}

#[test]
fn latency_far_above_the_baseline_shrinks_the_limit() {
    let mut control = Vegas::new(Bounds::new(1, 100), 20);
    control.observe(Sample::ok(1_000_000, 20));
    let grown = control.limit();

    control.observe(Sample::ok(50_000_000, grown));

    assert!(
        control.limit() < grown,
        "{} should be below {grown}",
        control.limit()
    );
}

#[test]
fn a_slow_dependency_at_a_steady_latency_does_not_shrink_the_limit() {
    const START: u32 = 10;
    const SAMPLES: u32 = 30;

    let mut control = Vegas::new(Bounds::new(1, 1_000), START);
    for _ in 0..SAMPLES {
        control.observe(Sample::ok(900_000_000, control.limit()));
    }

    assert_eq!(control.limit(), START + SAMPLES);
}

#[test]
fn an_overload_halves_the_limit_without_touching_the_baseline() {
    let mut control = Vegas::new(Bounds::new(1, 100), 40);
    control.observe(Sample::ok(1_000_000, 40));
    control.observe(Sample::overload(9_999_999_999, 41));

    assert_eq!(control.limit(), 20);
    assert_eq!(control.best_rtt_ns(), Some(1_000_000));
}
