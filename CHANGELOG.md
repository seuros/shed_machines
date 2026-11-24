# Changelog

## [0.5.2](https://github.com/seuros/shed_machines/compare/shed_machines/v0.5.1...shed_machines/v0.5.2) (2026-11-24)


### Features

* add timed retry-budget decay ([5a29294](https://github.com/seuros/shed_machines/commit/5a2929418b70e70fad2048edc443af1f4abb55a2))

## [0.5.1](https://github.com/seuros/shed_machines/compare/shed_machines/v0.5.0...shed_machines/v0.5.1) (2026-11-16)


### Bug Fixes

* fix wrapped classes at load so building a value runs no Ruby ([f38d7f4](https://github.com/seuros/shed_machines/commit/f38d7f4c5e00d6c9ce02acf83fb378c7a77bcac5))

## [0.5.0](https://github.com/seuros/shed_machines/compare/shed_machines/v0.4.1...shed_machines/v0.5.0) (2026-11-16)


### ⚠ BREAKING CHANGES

* seal Aimd, Vegas, Codel and Budget against subclassing

### Features

* seal Aimd, Vegas, Codel and Budget against subclassing ([e394613](https://github.com/seuros/shed_machines/commit/e394613ee8bb11a8e822895ce834f01c716b61c0))

## [0.4.1](https://github.com/seuros/shed_machines/compare/shed_machines/v0.4.0...shed_machines/v0.4.1) (2026-11-14)


### Bug Fixes

* **deps:** require matryona-machines &gt;= 0.4 ([f8035e7](https://github.com/seuros/shed_machines/commit/f8035e7578edc37d6de25111632b05d2c7d29da6))

## [0.4.0](https://github.com/seuros/shed_machines/compare/shed_machines/v0.3.0...shed_machines/v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* read Budget multipliers natively with matryona 0.3 finite

### Features

* read Budget multipliers natively with matryona 0.3 finite ([e973d9d](https://github.com/seuros/shed_machines/commit/e973d9d6a2614e57c284ffdfe13e7d8d2444b3b3))

## [0.3.0](https://github.com/seuros/shed_machines/compare/shed_machines/v0.2.0...shed_machines/v0.3.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* adopt matryona-machines 0.2: struct-only field reads, symbols! tables, caret rb-sys, missing_docs

### Features

* adopt matryona-machines 0.2: struct-only field reads, symbols! tables, caret rb-sys, missing_docs ([2d89f8f](https://github.com/seuros/shed_machines/commit/2d89f8f6f3214e0ff813e29a3b85b41eb5e1a98b))

## [0.2.0](https://github.com/seuros/shed_machines/compare/shed_machines/v0.1.3...shed_machines/v0.2.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* Duration controls, one Refusal, sealed validated values, exact-class native boundary

### Features

* Duration controls, one Refusal, sealed validated values, exact-class native boundary ([bfb04ad](https://github.com/seuros/shed_machines/commit/bfb04adb92f16fcaf9fa156f243ddaf8e0417c01))


### Bug Fixes

* keep test modules out of the packaged crate ([254cff8](https://github.com/seuros/shed_machines/commit/254cff86e3614d68be67cd1f3b1370cdcaffe096))

## [0.1.3](https://github.com/seuros/shed_machines/compare/shed_machines/v0.1.2...shed_machines/v0.1.3) (2026-11-01)


### Bug Fixes

* enable the rb-sys compiled fallback so new Rubies can build ([a5adf0a](https://github.com/seuros/shed_machines/commit/a5adf0a30c5db907c5b7c92b03f1d9e0b9c2fcf3))

## [0.1.2](https://github.com/seuros/shed_machines/compare/shed_machines/v0.1.1...shed_machines/v0.1.2) (2026-11-01)


### Bug Fixes

* a republished platform gem is skipped, not fatal ([35352a1](https://github.com/seuros/shed_machines/commit/35352a16dd73e19cf7e7661ce362f96e016a2cac))
* window the Vegas baseline so it cannot pin to one lucky call ([9cea8fa](https://github.com/seuros/shed_machines/commit/9cea8fa82f1fd9ab99f0bcf7a67a36ee24cd61ec))

## [0.1.1](https://github.com/seuros/shed_machines/compare/shed_machines-v0.1.0...shed_machines/v0.1.1) (2026-11-01)


### Features

* adaptive concurrency limits and load shedding ([e07f0dc](https://github.com/seuros/shed_machines/commit/e07f0dca6cc3dc65a49757d5eda0b96256310c61))
* pass a rubygems OTP through the environment ([8a4d15b](https://github.com/seuros/shed_machines/commit/8a4d15bc46fed686dd67da5566b8d5c252de617c))
* Ruby gem and magnus FFI for the control loops ([b323351](https://github.com/seuros/shed_machines/commit/b323351d11a2ee01782e93cafe769a24e4485ed0))


### Bug Fixes

* a configured CoDel duration goes through the same check as an observed one ([c424040](https://github.com/seuros/shed_machines/commit/c424040c95e419f88d20f06d19cd07bf5fba30f5))
* a source install can build, and a failed gem push fails the release ([4975e7f](https://github.com/seuros/shed_machines/commit/4975e7fcc39820ec681d79e8d0e3213878c61487))
* AIMD divided by zero, and both controllers saturated before dividing ([67006bd](https://github.com/seuros/shed_machines/commit/67006bdad51f9caef02594f042f2b5a5c7d34735))
* avoid re-entrant borrow and u32 clipping in shed control ([d4a1b09](https://github.com/seuros/shed_machines/commit/d4a1b0950e6de32bb2756f5ef183e649ff04d829))
* collect the built gems whatever depth the download puts them at ([7ffc2db](https://github.com/seuros/shed_machines/commit/7ffc2db96f97a1b30b3992b16d30dc45fd43f216))
* refuse a fractional call count instead of truncating it to zero ([869c3d8](https://github.com/seuros/shed_machines/commit/869c3d8988ae4e1be5924899466522053fa47981))
* reject durations and bounds the arithmetic cannot survive ([37edc6f](https://github.com/seuros/shed_machines/commit/37edc6f8849e9fff6192955cc41e910a930b3fcf))
* ship the core's test files so the packaged crate builds ([9a3e00d](https://github.com/seuros/shed_machines/commit/9a3e00dffffff31bbebfbaf6e9c57e2d21cea43e))
* the release tag carries the gem name, and the gate uses the aggregate output ([f0937ad](https://github.com/seuros/shed_machines/commit/f0937ad7ecbab770aaa81378c1035f80442291b7))

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
