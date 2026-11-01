# Changelog

## [0.2.0](https://github.com/seuros/shed_machines/compare/shed-machines-v0.1.0...shed-machines-v0.2.0) (2026-11-01)


### Features

* adaptive concurrency limits and load shedding ([e07f0dc](https://github.com/seuros/shed_machines/commit/e07f0dca6cc3dc65a49757d5eda0b96256310c61))
* Ruby gem and magnus FFI for the control loops ([b323351](https://github.com/seuros/shed_machines/commit/b323351d11a2ee01782e93cafe769a24e4485ed0))


### Bug Fixes

* AIMD divided by zero, and both controllers saturated before dividing ([67006bd](https://github.com/seuros/shed_machines/commit/67006bdad51f9caef02594f042f2b5a5c7d34735))
* avoid re-entrant borrow and u32 clipping in shed control ([d4a1b09](https://github.com/seuros/shed_machines/commit/d4a1b0950e6de32bb2756f5ef183e649ff04d829))
* ship the core's test files so the packaged crate builds ([9a3e00d](https://github.com/seuros/shed_machines/commit/9a3e00dffffff31bbebfbaf6e9c57e2d21cea43e))
