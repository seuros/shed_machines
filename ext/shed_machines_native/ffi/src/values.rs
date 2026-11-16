//! The `Data` values: `Bounds`, `Sample`, `Backoff` and `Band`.
//!
//! Each validates at construction through a private hook here, so a value
//! built by hand is held to the core constructor's rule. The binding trusts
//! no value it reads back: each must be an instance of exactly the gem's class
//! (a subclass constant leaks past the seal, so `is_a?` is not enough) and is
//! rebuilt through the core constructor, which `Marshal.load` skipped.

use magnus::{Error, IntoValue, RModule, Ruby, Symbol, Value, function, prelude::*};
use matryona_machines::ruby::{Class, Symbols, duration, exact_fields, integer, nanoseconds};
use shed_machines::{Backoff, Band, Bounds, Outcome, Sample};

use crate::rejected;

/// The `Data` classes, resolved once at load and pinned: the exact-class
/// check compares against these, and building an answer looks nothing up by
/// name.
struct Classes {
    bounds: Class,
    sample: Class,
    backoff: Class,
    band: Class,
}

matryona_machines::registry! {
    static CLASSES: Classes = "shed_machines";
    fn classes;
}

fn bounds(ruby: &Ruby, min: Value, max: Value) -> Result<Bounds, Error> {
    let (min, max) = (integer(min, "min")?, integer(max, "max")?);
    Bounds::new(min, max).ok_or_else(|| rejected(ruby, "bounds need 1 <= min <= max"))
}

fn backoff(ruby: &Ruby, kept: Value, of: Value) -> Result<Backoff, Error> {
    let (kept, of) = (integer(kept, "kept")?, integer(of, "of")?);
    Backoff::new(kept, of).ok_or_else(|| rejected(ruby, "a backoff needs kept < of"))
}

fn band(ruby: &Ruby, alpha: Value, beta: Value) -> Result<Band, Error> {
    let (alpha, beta) = (integer(alpha, "alpha")?, integer(beta, "beta")?);
    Band::new(alpha, beta).ok_or_else(|| rejected(ruby, "a band needs alpha <= beta"))
}

matryona_machines::symbols! {
    /// A sample's outcome, `:ok` or `:overload`: `TypeError` for a
    /// non-Symbol, `RangeError` for another Symbol.
    Outcomes for Outcome as "outcome" {
        Ok => "ok",
        Overload => "overload",
    }
}

fn outcome_symbol(ruby: &Ruby, outcome: Outcome) -> Symbol {
    Outcomes::symbol(ruby, outcome)
}

fn sample(ruby: &Ruby, rtt: Value, in_flight: Value, value: Value) -> Result<Sample, Error> {
    let (rtt, in_flight) = (duration(rtt, "rtt")?, integer(in_flight, "in_flight")?);
    let outcome = Outcomes::parse(ruby, value)?;
    Sample::new(rtt, in_flight, outcome)
        .ok_or_else(|| rejected(ruby, "a sample needs at least one call in flight"))
}

// Each value is an instance of exactly the gem's class (a subclass is
// refused, an opened singleton class is not), its fields read from the
// struct itself: a redefined reader is never called.

pub fn bounds_from(ruby: &Ruby, value: Value) -> Result<Bounds, Error> {
    let class = classes(ruby)?.bounds.get(ruby);
    let [min, max] = exact_fields(ruby, value, class, "expected a ShedMachines::Bounds")?;
    bounds(ruby, min, max)
}

pub fn backoff_from(ruby: &Ruby, value: Value) -> Result<Backoff, Error> {
    let class = classes(ruby)?.backoff.get(ruby);
    let [kept, of] = exact_fields(ruby, value, class, "expected a ShedMachines::Backoff")?;
    backoff(ruby, kept, of)
}

pub fn band_from(ruby: &Ruby, value: Value) -> Result<Band, Error> {
    let class = classes(ruby)?.band.get(ruby);
    let [alpha, beta] = exact_fields(ruby, value, class, "expected a ShedMachines::Band")?;
    band(ruby, alpha, beta)
}

pub fn sample_from(ruby: &Ruby, value: Value) -> Result<Sample, Error> {
    let class = classes(ruby)?.sample.get(ruby);
    let message = "expected a ShedMachines::Sample";
    let [rtt, in_flight, outcome] = exact_fields(ruby, value, class, message)?;
    sample(ruby, rtt, in_flight, outcome)
}

pub fn bounds_value(ruby: &Ruby, bounds: Bounds) -> Result<Value, Error> {
    classes(ruby)?
        .bounds
        .get(ruby)
        .funcall("new", (bounds.min(), bounds.max()))
}

pub fn backoff_value(ruby: &Ruby, backoff: Backoff) -> Result<Value, Error> {
    classes(ruby)?
        .backoff
        .get(ruby)
        .funcall("new", (backoff.kept(), backoff.of()))
}

pub fn band_value(ruby: &Ruby, band: Band) -> Result<Value, Error> {
    classes(ruby)?
        .band
        .get(ruby)
        .funcall("new", (band.alpha(), band.beta()))
}

/// What `Bounds.new` stores.
fn checked_bounds(ruby: &Ruby, min: Value, max: Value) -> Result<(u32, u32), Error> {
    bounds(ruby, min, max).map(|bounds| (bounds.min(), bounds.max()))
}

/// What `Backoff.new` stores.
fn checked_backoff(ruby: &Ruby, kept: Value, of: Value) -> Result<(u32, u32), Error> {
    backoff(ruby, kept, of).map(|backoff| (backoff.kept(), backoff.of()))
}

/// What `Band.new` stores.
fn checked_band(ruby: &Ruby, alpha: Value, beta: Value) -> Result<(u32, u32), Error> {
    band(ruby, alpha, beta).map(|band| (band.alpha(), band.beta()))
}

/// What `Sample.new` stores.
fn checked_sample(
    ruby: &Ruby,
    rtt: Value,
    in_flight: Value,
    outcome: Value,
) -> Result<(Value, u32, Value), Error> {
    sample(ruby, rtt, in_flight, outcome).map(|sample| {
        let outcome = outcome_symbol(ruby, sample.outcome()).into_value_with(ruby);
        let rtt = nanoseconds(ruby, sample.rtt()).as_value();
        (rtt, sample.in_flight(), outcome)
    })
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    // Held past this call: pinned so GC compaction cannot move them.
    let class = |name| Class::resolve(ruby, module, name);
    let classes = Classes {
        bounds: class("Bounds")?,
        sample: class("Sample")?,
        backoff: class("Backoff")?,
        band: class("Band")?,
    };
    CLASSES.set(ruby, classes)?;
    module.define_singleton_method("__bounds", function!(checked_bounds, 2))?;
    module.define_singleton_method("__backoff", function!(checked_backoff, 2))?;
    module.define_singleton_method("__band", function!(checked_band, 2))?;
    module.define_singleton_method("__sample", function!(checked_sample, 3))
}
