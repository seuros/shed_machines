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
mod init;
mod values;
mod vegas;

use std::cell::RefCell;

use magnus::{Error, RModule, Ruby, Symbol, Value, prelude::*};
use matryona_machines::ruby::Symbols;
use shed_machines::{Controller, Refusal, Verdict};

/// `RangeError` for a value the core's constructor answered `None` to.
pub(crate) fn rejected(ruby: &Ruby, why: &str) -> Error {
    Error::new(ruby.exception_range_error(), why.to_owned())
}

matryona_machines::symbols! {
    /// The core's refusals, each the `snake_case` of its variant.
    Refusals for Refusal as "a refusal" {
        TimeRegression => "time_regression",
        Overflow => "overflow",
    }
}

matryona_machines::symbols! {
    /// What `Codel#admit` decides.
    Verdicts for Verdict as "a verdict" {
        Admit => "admit",
        Shed => "shed",
    }
}

/// The honoured answer's symbol, or the `snake_case` symbol of the refusal.
pub(crate) fn answer<T>(
    ruby: &Ruby,
    answer: Result<T, Refusal>,
    honoured: impl FnOnce(T) -> Symbol,
) -> Value {
    match answer {
        Ok(value) => honoured(value),
        Err(refusal) => Refusals::symbol(ruby, refusal),
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
    Verdicts::symbol(ruby, verdict)
}

/// `ShedMachines`, where the `Data` classes live.
pub(crate) fn module(ruby: &Ruby) -> Result<RModule, Error> {
    ruby.define_module("ShedMachines")
}
