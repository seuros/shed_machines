//! `ShedMachines::Codel`: shed by how long work waited.

use std::cell::RefCell;

use magnus::typed_data::Obj;
use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*};
use shed_machines::Codel;
use shed_machines::codel::{DEFAULT_INTERVAL, DEFAULT_TARGET};

use crate::{Nanos, nanos, rejected, verdict};

#[magnus::wrap(class = "ShedMachines::Codel", free_immediately, size)]
struct Wrapper(RefCell<Codel>);

impl Wrapper {
    /// `target` and `interval` in Integer nanoseconds.
    fn new(ruby: &Ruby, Nanos(target): Nanos, Nanos(interval): Nanos) -> Result<Self, Error> {
        let queue = Codel::new(target, interval)
            .ok_or_else(|| rejected(ruby, "target and interval must not be zero"))?;
        Ok(Self(RefCell::new(queue)))
    }

    fn default() -> Self {
        Self(RefCell::new(Codel::default()))
    }

    /// `sojourn` is how long this work waited, `now` the current instant, both
    /// Integer nanoseconds. Answers `:admit`, `:shed` or `:time_regression`.
    fn admit(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        Nanos(sojourn): Nanos,
        Nanos(now): Nanos,
    ) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let answer = rb_self.0.borrow_mut().admit(sojourn, now);
        Ok(crate::answer(ruby, answer, |decided| {
            verdict(ruby, decided)
        }))
    }

    fn target(&self) -> u64 {
        nanos(self.0.borrow().target())
    }

    fn interval(&self) -> u64 {
        nanos(self.0.borrow().interval())
    }

    fn shedding(&self) -> bool {
        self.0.borrow().shedding()
    }

    fn shed_count(&self) -> u64 {
        self.0.borrow().shed_count()
    }

    fn inspect(&self) -> String {
        format!(
            "#<ShedMachines::Codel target={} shedding={}>",
            self.target(),
            self.shedding()
        )
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Codel", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("DEFAULT_TARGET", nanos(DEFAULT_TARGET))?;
    class.const_set("DEFAULT_INTERVAL", nanos(DEFAULT_INTERVAL))?;
    class.define_singleton_method("new", function!(Wrapper::new, 2))?;
    class.define_singleton_method("default", function!(Wrapper::default, 0))?;
    class.define_method("admit", method!(Wrapper::admit, 2))?;
    class.define_method("target", method!(Wrapper::target, 0))?;
    class.define_method("interval", method!(Wrapper::interval, 0))?;
    class.define_method("shedding?", method!(Wrapper::shedding, 0))?;
    class.define_method("shed_count", method!(Wrapper::shed_count, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
