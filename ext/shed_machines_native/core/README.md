# shed-machines

Adaptive concurrency limits and load shedding. `no_std`, allocation-free, zero
dependencies.

Nothing here enforces anything. These are the control loops. The caller owns the
semaphore, the queue, and the dice.

## Moving a concurrency limit

A fixed bulkhead is a guess that is wrong at every load except the one it was
tuned for. A controller watches completed calls and moves the limit.

```rust
use core::time::Duration;
use shed_machines::vegas::DEFAULT_BASELINE_WINDOW;
use shed_machines::{Band, Bounds, Controller, Outcome, Sample, Vegas};

let bounds = Bounds::new(1, 200).ok_or("bounds")?;
let mut control =
    Vegas::new(bounds, 10, Band::default(), DEFAULT_BASELINE_WINDOW).ok_or("config")?;

// The first call establishes the baseline latency.
control.observe(Sample::new(Duration::from_millis(1), 10, Outcome::Ok).ok_or("sample")?);

// Latency far above that baseline means calls are queueing, not working.
control.observe(Sample::new(Duration::from_millis(40), 11, Outcome::Ok).ok_or("sample")?);
assert!(control.limit() < 11);
# Ok::<(), &str>(())
```

`Vegas` infers queue depth from the gap between current latency and the smallest
seen in the last window or two, because latency alone cannot tell a slow
dependency from a deep queue. `Aimd` is the simpler choice when there is no
usable latency signal: it grows by one per saturated success and keeps a
`Backoff` fraction of the limit on overload.

`Vegas` needs homogeneous work. Over a mix whose service times differ by orders
of magnitude the baseline settles on the fastest kind, every slower call reads
as a deep queue, and the limit sinks to the floor and stays there; no `alpha`
and `beta` fix that, because the signal is measuring the wrong thing. Use `Aimd`
there, or one `Vegas` per class of work. The baseline itself is windowed over
`baseline_window` samples (`DEFAULT_BASELINE_WINDOW` is 1000), so one lucky call
cannot pin it for the life of the process.

## Shedding by waiting time

Queue length is the wrong signal, since a fast queue can be long and a stalled
one short. CoDel watches how long work waited and sheds only once that stays
above target for a full interval, so bursts pass and standing queues do not.

```rust
use core::time::Duration;
use shed_machines::{Codel, Refusal, Verdict};

let ms = Duration::from_millis;
let mut queue = Codel::default();

assert_eq!(queue.admit(ms(20), ms(0)), Ok(Verdict::Admit));
assert_eq!(queue.admit(ms(20), ms(150)), Ok(Verdict::Shed));
assert_eq!(queue.admit(ms(20), ms(149)), Err(Refusal::TimeRegression));
assert_eq!(queue.admit(ms(1), ms(200)), Ok(Verdict::Admit));
```

## Not amplifying an outage

A client that keeps retrying a failing dependency multiplies the load that broke
it. `Budget` reports what fraction of the next wave to drop locally.

```rust
use shed_machines::Budget;

let mut budget = Budget::default();
for _ in 0..1_000 {
    budget.requested()?;
}

assert!(budget.shed_probability() > 0.9);
budget.decay();
# Ok::<(), shed_machines::Refusal>(())
```

`shed_probability` is a fraction rather than a decision, so the core needs no
random number generator.

## Contract

### Values are valid or do not exist

`Bounds::new(min, max)` needs `1 <= min <= max`; `Sample::new(rtt, in_flight,
outcome)` needs `in_flight >= 1`; `Backoff::new(kept, of)` needs `kept < of`;
`Band::new(alpha, beta)` needs `alpha <= beta`; `Aimd::new` and `Vegas::new`
need the starting limit inside the bounds (and `Vegas` a nonzero
`baseline_window`); `Codel::new` needs a nonzero target and interval;
`Budget::new` and `Budget::restored` need a finite multiplier of at least
`LOWEST_MULTIPLIER`. Each answers `None` otherwise. Every field is private and
read through a method, so a value that exists is one the arithmetic can hold.

### Refusals

`Refusal` has two variants, and a refused call changes nothing: not a count, not
a recorded instant.

| Variant | When |
| --- | --- |
| `TimeRegression` | `Codel::admit` was given a `now` before the latest instant an honoured call recorded. |
| `Overflow` | `Budget::requested` or `Budget::accepted` found its count at `u64::MAX`. |

`Controller::observe` and `Budget::decay` never refuse: a `Sample` that exists
is valid, the limit saturates inside its `Bounds`, and halving cannot fail.

### Shedding is an answer, not a refusal

`Codel::admit` asks a well-formed question: should this work, which waited this
long, still run? `Verdict::Shed` is the controller's answer, and the call is
honoured: it records the instant, counts the shed and schedules the next one,
because that bookkeeping is the control law's state. `Verdict` therefore holds
only honoured outcomes. A `Refusal` is kept for a call that cannot be answered
at all.

### Time

Every instant and span is a `core::time::Duration` since an epoch the caller
picks: `Sample::rtt`, `Codel`'s `target`, `interval`, `sojourn` and `now`, and
`Vegas::best_rtt`. Regression is judged against the latest *honoured* instant;
a refused instant is not remembered, so the same call with a later `now` may be
honoured.

### Which answers record the clock

| Call | Answer | Records `now` |
| --- | --- | --- |
| `Codel::admit` | `Ok(Admit)` for work under target | no: it holds whatever the time |
| `Codel::admit` | `Ok(Admit)` or `Ok(Shed)` for work at or above target | yes |
| `Codel::admit` | `Err(TimeRegression)` | no |

Nothing else takes a clock.

### Repeats, scopes and restarts

No call here asks for an effect that can already hold, so there are no repeats
to honour. Nothing issues a handle, so there are no scopes. A `Budget` survives
a restart through `Budget::restored(multiplier, requests, accepts)`; the
controllers and `Codel` start fresh, which is what a new process should do with
a latency baseline.

## License

MIT.
