//! Ruby bindings for the `shed-machines` control laws.
//!
//! Durations cross this boundary as seconds and move through the control laws
//! as nanoseconds. [`to_ns`] and [`to_seconds`] are the only places either
//! representation changes.

#![allow(clippy::needless_pass_by_value)]

mod aimd;
mod budget;
mod codel;
mod vegas;

use std::cell::RefCell;

use magnus::scan_args::{get_kwargs, scan_args};
use magnus::{Error, Integer, RHash, Ruby, Symbol, TryConvert, Value, prelude::*};
use shed_machines::{Bounds, Controller, DEFAULT_CEILING, LOWEST_FLOOR, Outcome, Sample, Verdict};

/// Nanoseconds in a second.
pub(crate) const NS_PER_SECOND: u32 = 1_000_000_000;

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn to_ns(seconds: f64) -> u64 {
    (seconds * f64::from(NS_PER_SECOND)).round() as u64
}

#[expect(clippy::cast_precision_loss)]
pub(crate) fn to_seconds(nanoseconds: u64) -> f64 {
    nanoseconds as f64 / f64::from(NS_PER_SECOND)
}

/// A whole number of things: a limit, a bound, a call count, a ratio's two
/// sides.
///
/// magnus reads an integer through Ruby's `NUM2*` family, which truncates a
/// `Float` towards zero instead of refusing it, so an argument that counts
/// rather than measures is read as one of these and anything but an `Integer`
/// is turned away. An `Integer` outside the target width still comes back as
/// the `RangeError` magnus raises for it. Seconds stay [`duration`]'s.
pub(crate) struct Whole<T>(pub(crate) T);

impl<T: TryConvert> TryConvert for Whole<T> {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(not_whole(value));
        }
        T::try_convert(value).map(Self)
    }
}

fn not_whole(value: Value) -> Error {
    let ruby = Ruby::get_with(value);
    Error::new(
        ruby.exception_type_error(),
        format!(
            "no implicit conversion of {} into Integer: {}",
            value.class(),
            value.inspect()
        ),
    )
}

/// Seconds as nanoseconds, rejecting a negative or non-finite duration.
pub(crate) fn duration(ruby: &Ruby, name: &str, seconds: f64) -> Result<u64, Error> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!("{name} must be finite and not negative, got {seconds}"),
        ));
    }
    Ok(to_ns(seconds))
}

/// The keywords every controller is opened with: a starting `limit:`, the
/// `min:` and `max:` it moves within, and the two values it tunes itself with.
pub(crate) fn opening(
    ruby: &Ruby,
    keywords: RHash,
    tuning: [&str; 2],
) -> Result<(Bounds, u32, [Option<u32>; 2]), Error> {
    let keywords = get_kwargs::<
        _,
        (Whole<u32>,),
        (
            Option<Whole<u32>>,
            Option<Whole<u32>>,
            Option<Whole<u32>>,
            Option<Whole<u32>>,
        ),
        (),
    >(keywords, &["limit"], &["min", "max", tuning[0], tuning[1]])?;
    let (Whole(limit),) = keywords.required;
    let (min, max, first, second) = keywords.optional;
    let floor = min.map_or(LOWEST_FLOOR, |Whole(min)| min);
    let ceiling = max.map_or(DEFAULT_CEILING, |Whole(max)| max);
    if floor < LOWEST_FLOOR || ceiling < floor {
        return Err(Error::new(
            ruby.exception_arg_error(),
            format!(
                "min must be at least {LOWEST_FLOOR} and max at least min, got {floor}..{ceiling}"
            ),
        ));
    }
    let tuning = [first, second].map(|given| given.map(|Whole(value)| value));
    Ok((Bounds::new(floor, ceiling), limit, tuning))
}

/// Folds one completed call, given as `rtt:` in seconds, `in_flight:` and
/// `outcome:`, into a controller.
pub(crate) fn observe<C: Controller>(
    ruby: &Ruby,
    control: &RefCell<C>,
    args: &[Value],
) -> Result<(), Error> {
    let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
    let (rtt, Whole(in_flight), outcome) = get_kwargs::<_, (f64, Whole<u32>, Value), (), ()>(
        args.keywords,
        &["rtt", "in_flight", "outcome"],
        &[],
    )?
    .required;
    let sample = Sample {
        rtt_ns: duration(ruby, "rtt", rtt)?,
        in_flight,
        outcome: self::outcome(ruby, outcome)?,
    };
    control.borrow_mut().observe(sample);
    Ok(())
}

fn outcome(ruby: &Ruby, value: Value) -> Result<Outcome, Error> {
    let name = Symbol::from_value(value).and_then(|symbol| symbol.name().ok());
    match name.as_deref() {
        Some("ok") => Ok(Outcome::Ok),
        Some("overload") => Ok(Outcome::Overload),
        _ => Err(Error::new(
            ruby.exception_arg_error(),
            format!("unknown outcome: {}", value.inspect()),
        )),
    }
}

pub(crate) fn verdict(ruby: &Ruby, verdict: Verdict) -> Symbol {
    match verdict {
        Verdict::Admit => ruby.to_symbol("admit"),
        Verdict::Shed => ruby.to_symbol("shed"),
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("ShedMachines")?;
    module.const_set("NS_PER_SECOND", NS_PER_SECOND)?;
    module.const_set("LOWEST_FLOOR", LOWEST_FLOOR)?;
    module.const_set("DEFAULT_CEILING", DEFAULT_CEILING)?;
    aimd::define(ruby, module)?;
    vegas::define(ruby, module)?;
    codel::define(ruby, module)?;
    budget::define(ruby, module)
}
