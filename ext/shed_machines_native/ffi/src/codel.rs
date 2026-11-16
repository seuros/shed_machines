//! `ShedMachines::Codel`: shed by how long work waited.

use std::cell::RefCell;

use magnus::typed_data::Obj;
use magnus::{Error, Integer, RModule, Ruby, Value, function, method, prelude::*};
use matryona_machines::ruby::{Class, duration, nanoseconds};
use shed_machines::Codel;
use shed_machines::codel::{DEFAULT_INTERVAL, DEFAULT_TARGET};

use crate::{rejected, verdict};

#[magnus::wrap(class = "ShedMachines::Codel", free_immediately, size)]
struct Wrapper(RefCell<Codel>);

impl Wrapper {
    /// `target` and `interval` in Integer nanoseconds.
    fn new(ruby: &Ruby, target: Value, interval: Value) -> Result<Self, Error> {
        let (target, interval) = (duration(target, "target")?, duration(interval, "interval")?);
        let queue = Codel::new(target, interval)
            .ok_or_else(|| rejected(ruby, "target and interval must not be zero"))?;
        Ok(Self(RefCell::new(queue)))
    }

    fn default() -> Self {
        Self(RefCell::new(Codel::default()))
    }

    /// `sojourn` is how long this work waited, `now` the current instant, both
    /// Integer nanoseconds. Answers `:admit`, `:shed` or `:time_regression`.
    fn admit(ruby: &Ruby, rb_self: Obj<Self>, sojourn: Value, now: Value) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let (sojourn, now) = (duration(sojourn, "sojourn")?, duration(now, "now")?);
        let answer = rb_self.0.borrow_mut().admit(sojourn, now);
        Ok(crate::answer(ruby, answer, |decided| {
            verdict(ruby, decided)
        }))
    }

    fn target(ruby: &Ruby, rb_self: &Self) -> Integer {
        nanoseconds(ruby, rb_self.0.borrow().target())
    }

    fn interval(ruby: &Ruby, rb_self: &Self) -> Integer {
        nanoseconds(ruby, rb_self.0.borrow().interval())
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
            self.0.borrow().target().as_nanos(),
            self.shedding()
        )
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Codel", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("DEFAULT_TARGET", nanoseconds(ruby, DEFAULT_TARGET))?;
    class.const_set("DEFAULT_INTERVAL", nanoseconds(ruby, DEFAULT_INTERVAL))?;
    class.define_singleton_method("new", function!(Wrapper::new, 2))?;
    class.define_singleton_method("default", function!(Wrapper::default, 0))?;
    class.define_method("admit", method!(Wrapper::admit, 2))?;
    class.define_method("target", method!(Wrapper::target, 0))?;
    class.define_method("interval", method!(Wrapper::interval, 0))?;
    class.define_method("shedding?", method!(Wrapper::shedding, 0))?;
    class.define_method("shed_count", method!(Wrapper::shed_count, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Class::pin_wrapped::<Wrapper>(ruby, class).map(drop)
}
