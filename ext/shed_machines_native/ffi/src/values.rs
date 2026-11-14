//! The `Data` values: `Bounds`, `Sample`, `Backoff` and `Band`.
//!
//! Each validates at construction through a private hook here, so a value
//! built by hand is held to the core constructor's rule. The binding trusts
//! no value it reads back: each must be an instance of exactly the gem's class
//! (a subclass constant leaks past the seal, so `is_a?` is not enough) and is
//! rebuilt through the core constructor, which `Marshal.load` skipped.

use std::sync::OnceLock;

use magnus::rb_sys::AsRawValue;
use magnus::value::Opaque;
use magnus::{Error, IntoValue, RClass, RModule, Ruby, Symbol, Value, function, prelude::*};
use shed_machines::{Backoff, Band, Bounds, Outcome, Sample};

use crate::{Nanos, Whole, nanos, rejected};

/// The `Data` classes, resolved once at load: the exact-class check compares
/// against these, and building an answer looks nothing up by name.
struct Classes {
    bounds: Opaque<RClass>,
    sample: Opaque<RClass>,
    backoff: Opaque<RClass>,
    band: Opaque<RClass>,
}

static CLASSES: OnceLock<Classes> = OnceLock::new();

fn classes(ruby: &Ruby) -> Result<&'static Classes, Error> {
    CLASSES.get().ok_or_else(|| {
        Error::new(
            ruby.exception_runtime_error(),
            "load shed_machines, not the extension alone",
        )
    })
}

/// `value`, if its class is exactly `class`; `TypeError` otherwise, for a
/// subclass too.
fn data(ruby: &Ruby, value: Value, class: Opaque<RClass>, name: &str) -> Result<Value, Error> {
    if value.class().as_raw() == ruby.get_inner(class).as_raw() {
        return Ok(value);
    }
    Err(Error::new(
        ruby.exception_type_error(),
        format!("expected a ShedMachines::{name}, got {}", value.inspect()),
    ))
}

fn bounds(ruby: &Ruby, Whole(min): Whole<u32>, Whole(max): Whole<u32>) -> Result<Bounds, Error> {
    Bounds::new(min, max).ok_or_else(|| rejected(ruby, "bounds need 1 <= min <= max"))
}

fn backoff(ruby: &Ruby, Whole(kept): Whole<u32>, Whole(of): Whole<u32>) -> Result<Backoff, Error> {
    Backoff::new(kept, of).ok_or_else(|| rejected(ruby, "a backoff needs kept < of"))
}

fn band(ruby: &Ruby, Whole(alpha): Whole<u32>, Whole(beta): Whole<u32>) -> Result<Band, Error> {
    Band::new(alpha, beta).ok_or_else(|| rejected(ruby, "a band needs alpha <= beta"))
}

/// `:ok` or `:overload`: `TypeError` for a non-Symbol, `RangeError` for
/// another Symbol.
fn outcome(ruby: &Ruby, value: Value) -> Result<Outcome, Error> {
    let Some(symbol) = Symbol::from_value(value) else {
        return Err(Error::new(
            ruby.exception_type_error(),
            format!("an outcome is a Symbol, got {}", value.inspect()),
        ));
    };
    match symbol.name()?.as_ref() {
        "ok" => Ok(Outcome::Ok),
        "overload" => Ok(Outcome::Overload),
        _ => Err(rejected(ruby, "an outcome is :ok or :overload")),
    }
}

fn outcome_symbol(ruby: &Ruby, outcome: Outcome) -> Symbol {
    ruby.to_symbol(match outcome {
        Outcome::Ok => "ok",
        Outcome::Overload => "overload",
    })
}

fn sample(
    ruby: &Ruby,
    Nanos(rtt): Nanos,
    Whole(in_flight): Whole<u32>,
    value: Value,
) -> Result<Sample, Error> {
    let outcome = outcome(ruby, value)?;
    Sample::new(rtt, in_flight, outcome)
        .ok_or_else(|| rejected(ruby, "a sample needs at least one call in flight"))
}

pub fn bounds_from(ruby: &Ruby, value: Value) -> Result<Bounds, Error> {
    let value = data(ruby, value, classes(ruby)?.bounds, "Bounds")?;
    bounds(ruby, value.funcall("min", ())?, value.funcall("max", ())?)
}

pub fn backoff_from(ruby: &Ruby, value: Value) -> Result<Backoff, Error> {
    let value = data(ruby, value, classes(ruby)?.backoff, "Backoff")?;
    backoff(ruby, value.funcall("kept", ())?, value.funcall("of", ())?)
}

pub fn band_from(ruby: &Ruby, value: Value) -> Result<Band, Error> {
    let value = data(ruby, value, classes(ruby)?.band, "Band")?;
    band(
        ruby,
        value.funcall("alpha", ())?,
        value.funcall("beta", ())?,
    )
}

pub fn sample_from(ruby: &Ruby, value: Value) -> Result<Sample, Error> {
    let value = data(ruby, value, classes(ruby)?.sample, "Sample")?;
    sample(
        ruby,
        value.funcall("rtt", ())?,
        value.funcall("in_flight", ())?,
        value.funcall("outcome", ())?,
    )
}

pub fn bounds_value(ruby: &Ruby, bounds: Bounds) -> Result<Value, Error> {
    ruby.get_inner(classes(ruby)?.bounds)
        .funcall("new", (bounds.min(), bounds.max()))
}

pub fn backoff_value(ruby: &Ruby, backoff: Backoff) -> Result<Value, Error> {
    ruby.get_inner(classes(ruby)?.backoff)
        .funcall("new", (backoff.kept(), backoff.of()))
}

pub fn band_value(ruby: &Ruby, band: Band) -> Result<Value, Error> {
    ruby.get_inner(classes(ruby)?.band)
        .funcall("new", (band.alpha(), band.beta()))
}

/// What `Bounds.new` stores.
fn checked_bounds(ruby: &Ruby, min: Whole<u32>, max: Whole<u32>) -> Result<(u32, u32), Error> {
    bounds(ruby, min, max).map(|bounds| (bounds.min(), bounds.max()))
}

/// What `Backoff.new` stores.
fn checked_backoff(ruby: &Ruby, kept: Whole<u32>, of: Whole<u32>) -> Result<(u32, u32), Error> {
    backoff(ruby, kept, of).map(|backoff| (backoff.kept(), backoff.of()))
}

/// What `Band.new` stores.
fn checked_band(ruby: &Ruby, alpha: Whole<u32>, beta: Whole<u32>) -> Result<(u32, u32), Error> {
    band(ruby, alpha, beta).map(|band| (band.alpha(), band.beta()))
}

/// What `Sample.new` stores.
fn checked_sample(
    ruby: &Ruby,
    rtt: Nanos,
    in_flight: Whole<u32>,
    outcome: Value,
) -> Result<(u64, u32, Value), Error> {
    sample(ruby, rtt, in_flight, outcome).map(|sample| {
        let outcome = outcome_symbol(ruby, sample.outcome()).into_value_with(ruby);
        (nanos(sample.rtt()), sample.in_flight(), outcome)
    })
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    // Held past this call: pinned so GC compaction cannot move them.
    let class = |name: &str| {
        let class = module.const_get::<_, RClass>(name)?;
        ruby.gc_register_mark_object(class);
        Ok::<_, Error>(Opaque::from(class))
    };
    let classes = Classes {
        bounds: class("Bounds")?,
        sample: class("Sample")?,
        backoff: class("Backoff")?,
        band: class("Band")?,
    };
    CLASSES.set(classes).map_err(|_| {
        Error::new(
            ruby.exception_runtime_error(),
            "the extension was loaded twice",
        )
    })?;
    module.define_singleton_method("__bounds", function!(checked_bounds, 2))?;
    module.define_singleton_method("__backoff", function!(checked_backoff, 2))?;
    module.define_singleton_method("__band", function!(checked_band, 2))?;
    module.define_singleton_method("__sample", function!(checked_sample, 3))
}
