//! `ShedMachines::Budget`: how much outbound work to drop before it leaves.

use std::cell::RefCell;

use magnus::scan_args::{get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::{Error, RHash, RModule, Ruby, Value, function, method, prelude::*};
use shed_machines::Budget;
use shed_machines::budget::DEFAULT_MULTIPLIER;

#[magnus::wrap(class = "ShedMachines::Budget", free_immediately, size)]
struct Wrapper(RefCell<Budget>);

impl Wrapper {
    fn new(args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (multiplier,) =
            get_kwargs::<_, (), (Option<f64>,), ()>(args.keywords, &[], &["multiplier"])?.optional;
        let budget = Budget::new(multiplier.unwrap_or(DEFAULT_MULTIPLIER));
        Ok(Self(RefCell::new(budget)))
    }

    fn requested(rb_self: Obj<Self>) -> Obj<Self> {
        rb_self.0.borrow_mut().requested();
        rb_self
    }

    fn accepted(rb_self: Obj<Self>) -> Obj<Self> {
        rb_self.0.borrow_mut().accepted();
        rb_self
    }

    /// Halves both counts, so the budget forgets an outage.
    fn decay(rb_self: Obj<Self>) -> Obj<Self> {
        rb_self.0.borrow_mut().decay();
        rb_self
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
    class.const_set("DEFAULT_MULTIPLIER", DEFAULT_MULTIPLIER)?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
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
