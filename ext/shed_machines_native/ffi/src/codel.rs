//! `ShedMachines::Codel`: shed by how long work waited.

use std::cell::RefCell;

use magnus::scan_args::{get_kwargs, scan_args};
use magnus::{Error, RHash, RModule, Ruby, Symbol, Value, function, method, prelude::*};
use shed_machines::Codel;
use shed_machines::codel::{DEFAULT_INTERVAL_NS, DEFAULT_TARGET_NS};

use crate::{to_ns, to_seconds, verdict};

#[magnus::wrap(class = "ShedMachines::Codel", free_immediately, size)]
struct Wrapper(RefCell<Codel>);

impl Wrapper {
    fn new(args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (target, interval) = get_kwargs::<_, (), (Option<f64>, Option<f64>), ()>(
            args.keywords,
            &[],
            &["target", "interval"],
        )?
        .optional;
        let queue = Codel::new(
            target.map_or(DEFAULT_TARGET_NS, to_ns),
            interval.map_or(DEFAULT_INTERVAL_NS, to_ns),
        );
        Ok(Self(RefCell::new(queue)))
    }

    /// `sojourn:` is how long this work waited, `now:` the current time, both
    /// in seconds. Answers `:admit` or `:shed`.
    fn admit(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<Symbol, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (sojourn, now) =
            get_kwargs::<_, (f64, f64), (), ()>(args.keywords, &["sojourn", "now"], &[])?.required;
        let answer = rb_self.0.borrow_mut().admit(to_ns(sojourn), to_ns(now));
        Ok(verdict(ruby, answer))
    }

    /// The acceptable waiting time, in seconds.
    fn interval(&self) -> f64 {
        to_seconds(self.0.borrow().interval_ns())
    }

    fn target(&self) -> f64 {
        to_seconds(self.0.borrow().target_ns())
    }

    fn shedding(&self) -> bool {
        self.0.borrow().shedding()
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
    class.const_set("DEFAULT_TARGET_NS", DEFAULT_TARGET_NS)?;
    class.const_set("DEFAULT_INTERVAL_NS", DEFAULT_INTERVAL_NS)?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("admit", method!(Wrapper::admit, -1))?;
    class.define_method("target", method!(Wrapper::target, 0))?;
    class.define_method("interval", method!(Wrapper::interval, 0))?;
    class.define_method("shedding?", method!(Wrapper::shedding, 0))?;
    class.define_method("inspect", method!(Wrapper::inspect, 0))?;
    Ok(())
}
