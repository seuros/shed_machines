use core::time::Duration;

use super::{Bounds, Outcome, Refusal, Sample};

/// Bounds the test knows are valid.
pub fn bounds(min: u32, max: u32) -> Bounds {
    Bounds::new(min, max).expect("the test's bounds")
}

/// A completed call that finished, `rtt_ns` nanoseconds after it started.
pub fn ok(rtt_ns: u64, in_flight: u32) -> Sample {
    sample(rtt_ns, in_flight, Outcome::Ok)
}

/// A completed call that reported overload.
pub fn overload(rtt_ns: u64, in_flight: u32) -> Sample {
    sample(rtt_ns, in_flight, Outcome::Overload)
}

fn sample(rtt_ns: u64, in_flight: u32, outcome: Outcome) -> Sample {
    Sample::new(Duration::from_nanos(rtt_ns), in_flight, outcome).expect("a call in flight")
}

/// xorshift64: enough randomness for a reference-model test, and repeatable.
pub struct Dice(pub u64);

impl Dice {
    pub const fn roll(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn bounds_exist_only_around_a_limit_and_clamp_both_ways() {
    assert_eq!(Bounds::new(0, 10), None);
    assert_eq!(Bounds::new(10, 9), None);
    assert_eq!(
        Bounds::new(1, 1).map(|bounds| (bounds.min(), bounds.max())),
        Some((1, 1))
    );
    assert_eq!(Bounds::new(1, 1_000), Some(Bounds::default()));

    let bounds = bounds(2, 10);
    for (limit, clamped) in [(0, 2), (1, 2), (2, 2), (5, 5), (10, 10), (11, 10), (99, 10)] {
        assert_eq!(bounds.clamp(limit), clamped, "{limit}");
        assert_eq!(bounds.contains(limit), limit == clamped, "{limit}");
    }
}

#[test]
fn a_sample_with_nothing_in_flight_does_not_exist() {
    assert_eq!(Sample::new(Duration::ZERO, 0, Outcome::Ok), None);
    assert_eq!(Sample::new(Duration::ZERO, 0, Outcome::Overload), None);

    let sample = ok(7, 3);
    assert_eq!(sample.rtt(), Duration::from_nanos(7));
    assert_eq!(sample.in_flight(), 3);
    assert_eq!(sample.outcome(), Outcome::Ok);
}

#[test]
fn every_refusal_says_why() {
    extern crate alloc;
    use alloc::string::ToString;

    for refusal in [Refusal::TimeRegression, Refusal::Overflow] {
        let error: &dyn core::error::Error = &refusal;
        assert_ne!(error.to_string(), "");
    }
}
