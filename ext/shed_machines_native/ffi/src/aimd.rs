//! `ShedMachines::Aimd`: grow by one per saturated success, cut on overload.

use std::cell::RefCell;

use magnus::scan_args::scan_args;
use magnus::typed_data::Obj;
use magnus::{Error, RHash, RModule, Ruby, Value, function, method, prelude::*};
use shed_machines::aimd::{DEFAULT_BACKOFF_KEPT, DEFAULT_BACKOFF_OF};
use shed_machines::{Aimd, Controller};

use crate::{observe, opening};

#[magnus::wrap(class = "ShedMachines::Aimd", free_immediately, size)]
struct Wrapper(RefCell<Aimd>);

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (bounds, limit, [kept, of]) = opening(ruby, args.keywords, ["kept", "of"])?;
        let control = Aimd::new(bounds, limit).backoff(
            kept.unwrap_or(DEFAULT_BACKOFF_KEPT),
            of.unwrap_or(DEFAULT_BACKOFF_OF),
        );
        Ok(Self(RefCell::new(control)))
    }

    fn observe(ruby: &Ruby, rb_self: Obj<Self>, args: &[Value]) -> Result<Obj<Self>, Error> {
        observe(ruby, &rb_self.0, args)?;
        Ok(rb_self)
    }

    fn limit(&self) -> u32 {
        self.0.borrow().limit()
    }

    fn inspect(&self) -> String {
        format!("#<ShedMachines::Aimd limit={}>", self.limit())
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Aimd", ruby.class_object())?;
    class.const_set("DEFAULT_BACKOFF_KEPT", DEFAULT_BACKOFF_KEPT)?;
    class.const_set("DEFAULT_BACKOFF_OF", DEFAULT_BACKOFF_OF)?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("observe", method!(Wrapper::observe, -1))?;
    class.define_method("limit", method!(Wrapper::limit, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
