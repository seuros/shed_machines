//! `ShedMachines::Vegas`: steer the limit by how many calls look queued.

use std::cell::RefCell;

use magnus::typed_data::Obj;
use magnus::{Error, Integer, RModule, Ruby, Value, function, method, prelude::*};
use matryona_machines::ruby::{Class, integer, nanoseconds};
use shed_machines::vegas::{
    DEFAULT_ALPHA, DEFAULT_BASELINE_WINDOW, DEFAULT_BETA, OVERLOAD_DIVISOR,
};
use shed_machines::{Controller, Vegas};

use crate::rejected;
use crate::values::{band_from, band_value, bounds_from, bounds_value};

#[magnus::wrap(class = "ShedMachines::Vegas", free_immediately, size)]
struct Wrapper(RefCell<Vegas>);

impl Wrapper {
    fn new(
        ruby: &Ruby,
        bounds: Value,
        limit: Value,
        band: Value,
        baseline_window: Value,
    ) -> Result<Self, Error> {
        let limit: u32 = integer(limit, "limit")?;
        let baseline_window: u32 = integer(baseline_window, "baseline_window")?;
        let (bounds, band) = (bounds_from(ruby, bounds)?, band_from(ruby, band)?);
        let control = Vegas::new(bounds, limit, band, baseline_window).ok_or_else(|| {
            rejected(
                ruby,
                "the starting limit must be inside the bounds and the baseline window not zero",
            )
        })?;
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

    fn band(ruby: &Ruby, rb_self: &Self) -> Result<Value, Error> {
        let band = rb_self.0.borrow().band();
        band_value(ruby, band)
    }

    fn baseline_window(&self) -> u32 {
        self.0.borrow().baseline_window()
    }

    /// The smallest round-trip time in the last window or two, in Integer
    /// nanoseconds; `nil` until a call has completed without overload.
    fn best_rtt(ruby: &Ruby, rb_self: &Self) -> Option<Integer> {
        let best = rb_self.0.borrow().best_rtt();
        best.map(|rtt| nanoseconds(ruby, rtt))
    }

    fn inspect(&self) -> String {
        let best_rtt = self
            .0
            .borrow()
            .best_rtt()
            .map_or_else(|| "nil".to_owned(), |rtt| rtt.as_nanos().to_string());
        format!(
            "#<ShedMachines::Vegas limit={} best_rtt={best_rtt}>",
            self.limit()
        )
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Vegas", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("DEFAULT_ALPHA", DEFAULT_ALPHA)?;
    class.const_set("DEFAULT_BETA", DEFAULT_BETA)?;
    class.const_set("OVERLOAD_DIVISOR", OVERLOAD_DIVISOR)?;
    class.const_set("DEFAULT_BASELINE_WINDOW", DEFAULT_BASELINE_WINDOW)?;
    class.define_singleton_method("new", function!(Wrapper::new, 4))?;
    class.define_method("observe", method!(Wrapper::observe, 1))?;
    class.define_method("limit", method!(Wrapper::limit, 0))?;
    class.define_method("bounds", method!(Wrapper::bounds, 0))?;
    class.define_method("band", method!(Wrapper::band, 0))?;
    class.define_method("baseline_window", method!(Wrapper::baseline_window, 0))?;
    class.define_method("best_rtt", method!(Wrapper::best_rtt, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Class::pin_wrapped::<Wrapper>(ruby, class).map(drop)
}
