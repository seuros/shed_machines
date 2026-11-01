//! `ShedMachines::Vegas`: steer the limit by how many calls look queued.

use std::cell::RefCell;

use magnus::scan_args::scan_args;
use magnus::typed_data::Obj;
use magnus::{Error, RHash, RModule, Ruby, Value, function, method, prelude::*};
use shed_machines::vegas::{
    DEFAULT_ALPHA, DEFAULT_BASELINE_WINDOW, DEFAULT_BETA, OVERLOAD_DIVISOR,
};
use shed_machines::{Controller, Vegas};

use crate::{Whole, observe, opening, to_seconds};

#[magnus::wrap(class = "ShedMachines::Vegas", free_immediately, size)]
struct Wrapper(RefCell<Vegas>);

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (bounds, limit, [alpha, beta]) = opening(ruby, args.keywords, ["alpha", "beta"])?;
        let control = Vegas::new(bounds, limit)
            .band(alpha.unwrap_or(DEFAULT_ALPHA), beta.unwrap_or(DEFAULT_BETA));
        Ok(Self(RefCell::new(control)))
    }

    fn observe(ruby: &Ruby, rb_self: Obj<Self>, args: &[Value]) -> Result<Obj<Self>, Error> {
        observe(ruby, &rb_self.0, args)?;
        Ok(rb_self)
    }

    fn limit(&self) -> u32 {
        self.0.borrow().limit()
    }

    /// Samples one baseline window spans. Chainable, like the Rust builder.
    fn baseline_window(rb_self: Obj<Self>, Whole(samples): Whole<u32>) -> Obj<Self> {
        let windowed = rb_self.0.borrow().baseline_window(samples);
        *rb_self.0.borrow_mut() = windowed;
        rb_self
    }

    /// The smallest round-trip time in the last window or two, in seconds.
    fn best_rtt(&self) -> Option<f64> {
        self.0.borrow().best_rtt_ns().map(to_seconds)
    }

    fn inspect(&self) -> String {
        let best_rtt = self
            .best_rtt()
            .map_or_else(|| "nil".to_owned(), |rtt| rtt.to_string());
        format!(
            "#<ShedMachines::Vegas limit={} best_rtt={best_rtt}>",
            self.limit()
        )
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Vegas", ruby.class_object())?;
    class.const_set("DEFAULT_ALPHA", DEFAULT_ALPHA)?;
    class.const_set("DEFAULT_BETA", DEFAULT_BETA)?;
    class.const_set("OVERLOAD_DIVISOR", OVERLOAD_DIVISOR)?;
    class.const_set("DEFAULT_BASELINE_WINDOW", DEFAULT_BASELINE_WINDOW)?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("observe", method!(Wrapper::observe, -1))?;
    class.define_method("limit", method!(Wrapper::limit, 0))?;
    class.define_method("baseline_window", method!(Wrapper::baseline_window, 1))?;
    class.define_method("best_rtt", method!(Wrapper::best_rtt, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
