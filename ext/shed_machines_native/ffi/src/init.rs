//! Private native entry point.
use magnus::{Error, Ruby, prelude::*};
use shed_machines::{DEFAULT_CEILING, LOWEST_FLOOR};

use crate::{aimd, budget, codel, module, values, vegas};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = module(ruby)?;
    module.const_set("LOWEST_FLOOR", LOWEST_FLOOR)?;
    module.const_set("DEFAULT_CEILING", DEFAULT_CEILING)?;
    values::define(ruby, module)?;
    aimd::define(ruby, module)?;
    vegas::define(ruby, module)?;
    codel::define(ruby, module)?;
    budget::define(ruby, module)
}
