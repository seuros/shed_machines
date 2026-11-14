use core::time::Duration;

use crate::tests::{bounds, ok, overload};
use crate::vegas::DEFAULT_BASELINE_WINDOW;
use crate::{Band, Controller, Vegas};

fn control(min: u32, max: u32, limit: u32) -> Vegas {
    windowed(min, max, limit, DEFAULT_BASELINE_WINDOW)
}

fn windowed(min: u32, max: u32, limit: u32, window: u32) -> Vegas {
    Vegas::new(bounds(min, max), limit, Band::default(), window).expect("a controller")
}

/// What `best_rtt` answers once it holds `count` nanoseconds.
#[expect(clippy::unnecessary_wraps, reason = "compared against an Option")]
const fn ns(count: u64) -> Option<Duration> {
    Some(Duration::from_nanos(count))
}

#[test]
fn the_first_sample_sets_the_baseline() {
    let mut control = control(1, 100, 10);
    assert_eq!(control.best_rtt(), None);

    control.observe(ok(5_000_000, 10));

    assert_eq!(control.best_rtt(), ns(5_000_000));
}

#[test]
fn latency_far_above_the_baseline_shrinks_the_limit() {
    let mut control = control(1, 100, 20);
    control.observe(ok(1_000_000, 20));
    let grown = control.limit();

    control.observe(ok(50_000_000, grown));

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

    let mut control = control(1, 1_000, START);
    for _ in 0..SAMPLES {
        control.observe(ok(900_000_000, control.limit()));
    }

    assert_eq!(control.limit(), START + SAMPLES);
}

#[test]
fn an_overload_halves_the_limit_without_touching_the_baseline() {
    let mut control = control(1, 100, 40);
    control.observe(ok(1_000_000, 40));
    control.observe(overload(9_999_999_999, 41));

    assert_eq!(control.limit(), 20);
    assert_eq!(control.best_rtt(), ns(1_000_000));
}

/// A baseline that only shrank would pin itself to the luckiest call ever made.
/// One fast sample followed by a dependency that settles ten times slower has
/// to stop reading as a deep queue once the windows have turned over, or the
/// limit sits at the floor for the life of the process.
#[test]
fn a_dependency_that_slows_down_for_good_earns_a_new_baseline() {
    const FAST_NS: u64 = 1_000_000;
    const SETTLED_NS: u64 = 10_000_000;

    let mut control = windowed(1, 100, 10, 8);
    control.observe(ok(FAST_NS, 10));
    assert_eq!(control.best_rtt(), ns(FAST_NS));

    // The fast sample is only evicted once the window holding it has been
    // retired and then replaced, so it drags the limit down until it goes.
    for _ in 0..16 {
        control.observe(ok(SETTLED_NS, control.limit()));
    }
    assert_eq!(control.best_rtt(), ns(SETTLED_NS));
    assert!(
        control.limit() < 10,
        "the stale baseline should have cost it"
    );

    // With an honest baseline the settled latency stops reading as a queue.
    for _ in 0..200 {
        control.observe(ok(SETTLED_NS, control.limit()));
    }
    assert_eq!(control.best_rtt(), ns(SETTLED_NS));
    assert_eq!(control.limit(), 100);
}

/// Inside a single window the baseline still holds, so a steady dependency is
/// not chasing its own latency upward sample by sample.
#[test]
fn the_baseline_holds_while_the_window_fills() {
    let mut control = windowed(1, 100, 10, 1_000);
    control.observe(ok(1_000_000, 10));
    for _ in 0..200 {
        control.observe(ok(20_000_000, control.limit()));
    }

    // Still the first window's minimum, 200 slower samples later.
    assert_eq!(control.best_rtt(), ns(1_000_000));
    // 19/20 of the limit reads as queued, so it falls to where that estimate
    // lands inside the hold band rather than chasing the latency upward.
    assert_eq!(control.limit(), 7);
}

#[test]
fn a_call_with_no_latency_to_report_carries_no_queue_signal() {
    let mut control = control(1, 100, 10);
    control.observe(ok(0, 10));

    assert_eq!(control.limit(), 11);
    assert_eq!(control.best_rtt(), ns(0));
}

#[test]
fn the_band_decides_when_to_hold() {
    let band = Band::new(1, 2).expect("1..=2 is a band");
    let mut control = Vegas::new(bounds(1, 100), 10, band, 1_000).expect("a controller");
    control.observe(ok(1_000_000, 10));
    control.observe(ok(1_500_000, 11));

    assert_eq!(control.limit(), 10);
    assert_eq!(control.band(), band);
    assert_eq!((band.alpha(), band.beta()), (1, 2));
    assert_eq!(control.baseline_window(), 1_000);
    assert_eq!(control.bounds(), bounds(1, 100));
}

#[test]
fn a_configuration_the_estimate_cannot_hold_is_not_a_controller() {
    assert_eq!(Band::new(4, 3), None);
    assert_eq!(Band::new(3, 6), Some(Band::default()));
    assert!(Band::new(3, 3).is_some());

    assert_eq!(Vegas::new(bounds(2, 10), 1, Band::default(), 8), None);
    assert_eq!(Vegas::new(bounds(2, 10), 11, Band::default(), 8), None);
    assert_eq!(Vegas::new(bounds(2, 10), 5, Band::default(), 0), None);
    assert!(Vegas::new(bounds(2, 10), 5, Band::default(), 1).is_some());
}

/// A window of one retires every sample at once, so the baseline is always
/// the latest round-trip time.
#[test]
fn a_window_of_one_follows_the_latest_call() {
    let mut control = windowed(1, 100, 10, 1);
    control.observe(ok(1_000_000, 10));
    control.observe(ok(9_000_000, 11));

    assert_eq!(control.best_rtt(), ns(9_000_000));
}
