//! `ShedMachines::Aimd`: grow by one per saturated success, cut on overload.

use std::cell::RefCell;

use magnus::typed_data::Obj;
use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*};
use shed_machines::aimd::{DEFAULT_BACKOFF_KEPT, DEFAULT_BACKOFF_OF};
use shed_machines::{Aimd, Controller};

use crate::values::{backoff_from, backoff_value, bounds_from, bounds_value};
use crate::{Whole, rejected};

#[magnus::wrap(class = "ShedMachines::Aimd", free_immediately, size)]
struct Wrapper(RefCell<Aimd>);

impl Wrapper {
    fn new(
        ruby: &Ruby,
        bounds: Value,
        Whole(limit): Whole<u32>,
        backoff: Value,
    ) -> Result<Self, Error> {
        let (bounds, backoff) = (bounds_from(ruby, bounds)?, backoff_from(ruby, backoff)?);
        let control = Aimd::new(bounds, limit, backoff)
            .ok_or_else(|| rejected(ruby, "the starting limit must be inside the bounds"))?;
        Ok(Self(RefCell::new(control)))
    }

    fn observe(ruby: &Ruby, rb_self: Obj<Self>, sample: Value) -> Result<(), Error> {
        crate::observe(ruby, rb_self.as_value(), &rb_self.0, sample)
    }

    fn limit(&self) -> u32 {
        self.0.borrow().limit()
    }

    fn bounds(ruby: &Ruby, rb_self: &Self) -> Result<Value, Error> {
        let bounds = rb_self.0.borrow().bounds();
        bounds_value(ruby, bounds)
    }

    fn backoff(ruby: &Ruby, rb_self: &Self) -> Result<Value, Error> {
        let backoff = rb_self.0.borrow().backoff();
        backoff_value(ruby, backoff)
    }

    fn inspect(&self) -> String {
        format!("#<ShedMachines::Aimd limit={}>", self.limit())
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Aimd", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("DEFAULT_BACKOFF_KEPT", DEFAULT_BACKOFF_KEPT)?;
    class.const_set("DEFAULT_BACKOFF_OF", DEFAULT_BACKOFF_OF)?;
    class.define_singleton_method("new", function!(Wrapper::new, 3))?;
    class.define_method("observe", method!(Wrapper::observe, 1))?;
    class.define_method("limit", method!(Wrapper::limit, 0))?;
    class.define_method("bounds", method!(Wrapper::bounds, 0))?;
    class.define_method("backoff", method!(Wrapper::backoff, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
