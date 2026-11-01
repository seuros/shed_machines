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

/// A baseline that only shrank would pin itself to the luckiest call ever made.
/// One fast sample followed by a dependency that settles ten times slower has
/// to stop reading as a deep queue once the windows have turned over, or the
/// limit sits at the floor for the life of the process.
#[test]
fn a_dependency_that_slows_down_for_good_earns_a_new_baseline() {
    const FAST_NS: u64 = 1_000_000;
    const SETTLED_NS: u64 = 10_000_000;

    let mut control = Vegas::new(Bounds::new(1, 100), 10).baseline_window(8);
    control.observe(Sample::ok(FAST_NS, 10));
    assert_eq!(control.best_rtt_ns(), Some(FAST_NS));

    // The fast sample is only evicted once the window holding it has been
    // retired and then replaced, so it drags the limit down until it goes.
    for _ in 0..16 {
        control.observe(Sample::ok(SETTLED_NS, control.limit()));
    }
    assert_eq!(control.best_rtt_ns(), Some(SETTLED_NS));
    assert!(
        control.limit() < 10,
        "the stale baseline should have cost it"
    );

    // With an honest baseline the settled latency stops reading as a queue.
    for _ in 0..200 {
        control.observe(Sample::ok(SETTLED_NS, control.limit()));
    }
    assert_eq!(control.best_rtt_ns(), Some(SETTLED_NS));
    assert_eq!(control.limit(), 100);
}

/// Inside a single window the baseline still holds, so a steady dependency is
/// not chasing its own latency upward sample by sample.
#[test]
fn the_baseline_holds_while_the_window_fills() {
    let mut control = Vegas::new(Bounds::new(1, 100), 10).baseline_window(1_000);
    control.observe(Sample::ok(1_000_000, 10));
    for _ in 0..200 {
        control.observe(Sample::ok(20_000_000, control.limit()));
    }

    // Still the first window's minimum, 200 slower samples later.
    assert_eq!(control.best_rtt_ns(), Some(1_000_000));
    // 19/20 of the limit reads as queued, so it falls to where that estimate
    // lands inside the hold band rather than chasing the latency upward.
    assert_eq!(control.limit(), 7);
}
