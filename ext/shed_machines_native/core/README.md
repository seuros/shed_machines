# shed-machines

Adaptive concurrency limits and load shedding. `no_std`, zero dependencies.

Nothing here enforces anything. These are the control loops. The caller owns the
semaphore, the queue, and the dice.

## Moving a concurrency limit

A fixed bulkhead is a guess that is wrong at every load except the one it was
tuned for. A controller watches completed calls and moves the limit.

```rust
use shed_machines::{Bounds, Controller, Sample, Vegas};

let mut control = Vegas::new(Bounds::new(1, 200), 10);

// The first call establishes the baseline latency.
control.observe(Sample::ok(1_000_000, 10));

// Latency far above that baseline means calls are queueing, not working.
control.observe(Sample::ok(40_000_000, 11));
assert!(control.limit() < 11);
```

`Vegas` infers queue depth from the gap between current latency and the smallest
seen in the last window or two, because latency alone cannot tell a slow
dependency from a deep queue. `Aimd` is the simpler choice when there is no
usable latency signal: it grows by one per saturated success and cuts by
`DEFAULT_BACKOFF` on overload.

`Vegas` needs homogeneous work. Over a mix whose service times differ by orders
of magnitude the baseline settles on the fastest kind, every slower call reads
as a deep queue, and the limit sinks to the floor and stays there; no `alpha`
and `beta` fix that, because the signal is measuring the wrong thing. Use `Aimd`
there, or one `Vegas` per class of work. The baseline itself is windowed over
`DEFAULT_BASELINE_WINDOW` samples, so one lucky call cannot pin it for the life
of the process — see `Vegas::baseline_window`.

## Shedding by waiting time

Queue length is the wrong signal, since a fast queue can be long and a stalled
one short. CoDel watches how long work waited and sheds only once that stays
above target for a full interval, so bursts pass and standing queues do not.

```rust
use shed_machines::{Codel, Verdict};

const MS: u64 = 1_000_000;
let mut queue = Codel::default();

assert_eq!(queue.admit(20 * MS, 0), Verdict::Admit);
assert_eq!(queue.admit(20 * MS, 150 * MS), Verdict::Shed);
assert_eq!(queue.admit(MS, 200 * MS), Verdict::Admit);
```

## Not amplifying an outage

A client that keeps retrying a failing dependency multiplies the load that broke
it. `Budget` reports what fraction of the next wave to drop locally.

```rust
use shed_machines::Budget;

let mut budget = Budget::default();
for _ in 0..1_000 {
    budget.requested();
}

assert!(budget.shed_probability() > 0.9);
budget.decay();
```

`shed_probability` is a fraction rather than a decision, so the core needs no
random number generator.

## License

MIT.
