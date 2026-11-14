//! Ruby bindings for the `shed-machines` control laws.
//!
//! Instants and spans cross this boundary as Integer nanoseconds and move
//! through the control laws as `Duration`. Everything that can raise, a type
//! check, a conversion, a class lookup, runs before a core call; after it the
//! binding only builds values from what the core answered. No `RefCell`
//! borrow is held while Ruby code runs: a reader copies the core value out
//! under a short borrow and only then calls `new`, so a `new` that re-enters
//! the controller makes an ordinary call instead of an uncatchable `fatal`.
//! No mutating call answers a Ruby object (only Symbols and `nil`), so each
//! mutates in place after its frozen check.

mod aimd;
mod budget;
mod codel;
mod values;
mod vegas;

use std::cell::RefCell;
use std::time::Duration;

use magnus::{Error, Integer, RModule, Ruby, Symbol, TryConvert, Value, prelude::*};
use shed_machines::{Controller, DEFAULT_CEILING, LOWEST_FLOOR, Refusal, Verdict};

/// A whole number of things: a limit, a bound, a call count, a ratio's two
/// sides.
///
/// magnus reads an integer through Ruby's `NUM2*` family, which truncates a
/// `Float` towards zero instead of refusing it, so an argument that counts
/// rather than measures is read as one of these and anything but an `Integer`
/// is a `TypeError`. An `Integer` outside the target width is the `RangeError`
/// magnus raises for it.
pub(crate) struct Whole<T>(pub(crate) T);

impl<T: TryConvert> TryConvert for Whole<T> {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            let ruby = Ruby::get_with(value);
            return Err(Error::new(
                ruby.exception_type_error(),
                format!(
                    "no implicit conversion of {} into Integer: {}",
                    value.class(),
                    value.inspect()
                ),
            ));
        }
        T::try_convert(value).map(Self)
    }
}

/// An instant or span given as Integer nanoseconds: `TypeError` for anything
/// but an `Integer`, `RangeError` for a negative one or one past 64 bits.
pub(crate) struct Nanos(pub(crate) Duration);

impl TryConvert for Nanos {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let Whole(nanos) = Whole::<u64>::try_convert(value)?;
        Ok(Self(Duration::from_nanos(nanos)))
    }
}

/// A `Duration` the binding was handed as nanoseconds, back as nanoseconds.
pub(crate) fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// `RangeError` for a value the core's constructor answered `None` to.
pub(crate) fn rejected(ruby: &Ruby, why: &str) -> Error {
    Error::new(ruby.exception_range_error(), why.to_owned())
}

/// The honoured answer's symbol, or the `snake_case` symbol of the refusal.
pub(crate) fn answer<T>(
    ruby: &Ruby,
    answer: Result<T, Refusal>,
    honoured: impl FnOnce(T) -> Symbol,
) -> Value {
    match answer {
        Ok(value) => honoured(value),
        Err(refusal) => ruby.to_symbol(match refusal {
            Refusal::TimeRegression => "time_regression",
            Refusal::Overflow => "overflow",
        }),
    }
    .as_value()
}

/// Folds a `ShedMachines::Sample` into `control`, converted before the limit
/// moves. Never refuses: answers `nil`.
pub(crate) fn observe<C: Controller>(
    ruby: &Ruby,
    this: Value,
    control: &RefCell<C>,
    sample: Value,
) -> Result<(), Error> {
    this.check_frozen()?;
    let sample = values::sample_from(ruby, sample)?;
    control.borrow_mut().observe(sample);
    Ok(())
}

pub(crate) fn verdict(ruby: &Ruby, verdict: Verdict) -> Symbol {
    ruby.to_symbol(match verdict {
        Verdict::Admit => "admit",
        Verdict::Shed => "shed",
    })
}

/// `ShedMachines`, where the `Data` classes live.
pub(crate) fn module(ruby: &Ruby) -> Result<RModule, Error> {
    ruby.define_module("ShedMachines")
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = module(ruby)?;
    module.const_set("LOWEST_FLOOR", LOWEST_FLOOR)?;
    module.const_set("DEFAULT_CEILING", DEFAULT_CEILING)?;
    values::define(ruby, module)?;
    aimd::define(ruby, module)?;
    vegas::define(ruby, module)?;
    codel::define(ruby, module)?;
    budget::define(ruby, module)
}
