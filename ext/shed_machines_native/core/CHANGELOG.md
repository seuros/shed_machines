# Changelog

## [0.6.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.5.0...shed-machines-v0.6.0) (2026-11-24)


### Features

* add timed retry-budget decay ([5a29294](https://github.com/seuros/shed_machines/commit/5a2929418b70e70fad2048edc443af1f4abb55a2))

## [0.5.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.4.0...shed-machines-v0.5.0) (2026-11-16)


### ⚠ BREAKING CHANGES

* seal Aimd, Vegas, Codel and Budget against subclassing

### Features

* seal Aimd, Vegas, Codel and Budget against subclassing ([e394613](https://github.com/seuros/shed_machines/commit/e394613ee8bb11a8e822895ce834f01c716b61c0))

## [0.4.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.3.0...shed-machines-v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* adopt matryona-machines 0.2: struct-only field reads, symbols! tables, caret rb-sys, missing_docs

### Features

* adopt matryona-machines 0.2: struct-only field reads, symbols! tables, caret rb-sys, missing_docs ([2d89f8f](https://github.com/seuros/shed_machines/commit/2d89f8f6f3214e0ff813e29a3b85b41eb5e1a98b))

## [0.3.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.2.1...shed-machines-v0.3.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* Duration controls, one Refusal, sealed validated values, exact-class native boundary

### Features

* Duration controls, one Refusal, sealed validated values, exact-class native boundary ([bfb04ad](https://github.com/seuros/shed_machines/commit/bfb04adb92f16fcaf9fa156f243ddaf8e0417c01))


### Bug Fixes

* keep test modules out of the packaged crate ([254cff8](https://github.com/seuros/shed_machines/commit/254cff86e3614d68be67cd1f3b1370cdcaffe096))

## [0.2.1](https://github.com/seuros/shed_machines/compare/shed-machines-v0.2.0...shed-machines-v0.2.1) (2026-11-01)


### Bug Fixes

* window the Vegas baseline so it cannot pin to one lucky call ([9cea8fa](https://github.com/seuros/shed_machines/commit/9cea8fa82f1fd9ab99f0bcf7a67a36ee24cd61ec))

## [0.2.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.1.0...shed-machines-v0.2.0) (2026-11-01)


### Features

* adaptive concurrency limits and load shedding ([e07f0dc](https://github.com/seuros/shed_machines/commit/e07f0dca6cc3dc65a49757d5eda0b96256310c61))
* Ruby gem and magnus FFI for the control loops ([b323351](https://github.com/seuros/shed_machines/commit/b323351d11a2ee01782e93cafe769a24e4485ed0))


### Bug Fixes

* AIMD divided by zero, and both controllers saturated before dividing ([67006bd](https://github.com/seuros/shed_machines/commit/67006bdad51f9caef02594f042f2b5a5c7d34735))
* avoid re-entrant borrow and u32 clipping in shed control ([d4a1b09](https://github.com/seuros/shed_machines/commit/d4a1b0950e6de32bb2756f5ef183e649ff04d829))
* ship the core's test files so the packaged crate builds ([9a3e00d](https://github.com/seuros/shed_machines/commit/9a3e00dffffff31bbebfbaf6e9c57e2d21cea43e))
