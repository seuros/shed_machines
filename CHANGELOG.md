# Changelog

## 0.1.0

- Initial release.
- `ShedMachines::Aimd`: additive increase per saturated success, multiplicative
  decrease on overload, with a configurable `kept / of` backoff fraction.
- `ShedMachines::Vegas`: TCP Vegas applied to a concurrency limit, steering by
  the queue estimate drawn from the gap between current and best round-trip
  time, with a configurable `alpha`/`beta` band.
- `ShedMachines::Codel`: controlled delay, shedding queued work by how long it
  waited once the wait stays above target for a full interval, with sheds that
  come faster as the inverse square root of how many have happened.
- `ShedMachines::Budget`: client-side adaptive throttling, reporting the
  fraction of the next retry wave to drop rather than making the decision.
- Durations cross the Ruby boundary as seconds in Floats and are converted once
  to nanoseconds; the control laws themselves are integer-only.
- `shed-machines` core crate: the four control laws in `no_std` Rust with zero
  dependencies. Builds and lints clean on bare metal.
- RBS signatures for the whole public API.
