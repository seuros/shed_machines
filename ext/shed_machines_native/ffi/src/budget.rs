//! `ShedMachines::Budget`: how much outbound work to drop before it leaves.

use std::cell::RefCell;

use magnus::typed_data::Obj;
use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*};
use matryona_machines::ruby::{finite, integer, symbol};
use shed_machines::budget::{DEFAULT_MULTIPLIER, LOWEST_MULTIPLIER};
use shed_machines::{Budget, Refusal};

use crate::{answer, rejected};

#[magnus::wrap(class = "ShedMachines::Budget", free_immediately, size)]
struct Wrapper(RefCell<Budget>);

fn multiplier_rejected(ruby: &Ruby) -> Error {
    rejected(
        ruby,
        "the multiplier must be finite and at least LOWEST_MULTIPLIER",
    )
}

/// `:counted`, or the refusal's symbol.
fn counted(ruby: &Ruby, counted: Result<(), Refusal>) -> Value {
    answer(ruby, counted, |()| symbol(ruby, "counted"))
}

impl Wrapper {
    fn new(ruby: &Ruby, multiplier: Value) -> Result<Self, Error> {
        let multiplier = finite(multiplier, "multiplier")?;
        let budget = Budget::new(multiplier).ok_or_else(|| multiplier_rejected(ruby))?;
        Ok(Self(RefCell::new(budget)))
    }

    fn restored(
        ruby: &Ruby,
        multiplier: Value,
        requests: Value,
        accepts: Value,
    ) -> Result<Self, Error> {
        let multiplier = finite(multiplier, "multiplier")?;
        let (requests, accepts) = (integer(requests, "requests")?, integer(accepts, "accepts")?);
        let budget = Budget::restored(multiplier, requests, accepts)
            .ok_or_else(|| multiplier_rejected(ruby))?;
        Ok(Self(RefCell::new(budget)))
    }

    fn default() -> Self {
        Self(RefCell::new(Budget::default()))
    }

    fn requested(ruby: &Ruby, rb_self: Obj<Self>) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let answer = rb_self.0.borrow_mut().requested();
        Ok(counted(ruby, answer))
    }

    fn accepted(ruby: &Ruby, rb_self: Obj<Self>) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let answer = rb_self.0.borrow_mut().accepted();
        Ok(counted(ruby, answer))
    }

    /// Halves both counts, so the budget forgets an outage. Answers `nil`.
    fn decay(rb_self: Obj<Self>) -> Result<(), Error> {
        rb_self.check_frozen()?;
        rb_self.0.borrow_mut().decay();
        Ok(())
    }

    fn multiplier(&self) -> f64 {
        self.0.borrow().multiplier()
    }

    fn requests(&self) -> u64 {
        self.0.borrow().requests()
    }

    fn accepts(&self) -> u64 {
        self.0.borrow().accepts()
    }

    /// The fraction of outbound work to drop locally, in `0.0..1.0`.
    fn shed_probability(&self) -> f64 {
        self.0.borrow().shed_probability()
    }

    fn inspect(&self) -> String {
        format!(
            "#<ShedMachines::Budget requests={} accepts={} shed_probability={}>",
            self.requests(),
            self.accepts(),
            self.shed_probability()
        )
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Budget", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("DEFAULT_MULTIPLIER", DEFAULT_MULTIPLIER)?;
    class.const_set("LOWEST_MULTIPLIER", LOWEST_MULTIPLIER)?;
    class.define_singleton_method("new", function!(Wrapper::new, 1))?;
    class.define_singleton_method("restored", function!(Wrapper::restored, 3))?;
    class.define_singleton_method("default", function!(Wrapper::default, 0))?;
    class.define_method("requested", method!(Wrapper::requested, 0))?;
    class.define_method("accepted", method!(Wrapper::accepted, 0))?;
    class.define_method("decay", method!(Wrapper::decay, 0))?;
    class.define_method("multiplier", method!(Wrapper::multiplier, 0))?;
    class.define_method("requests", method!(Wrapper::requests, 0))?;
    class.define_method("accepts", method!(Wrapper::accepts, 0))?;
    class.define_method("shed_probability", method!(Wrapper::shed_probability, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
