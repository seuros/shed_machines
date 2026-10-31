# ShedMachines

Adaptive concurrency limits and load shedding.

A fixed bulkhead is a guess that is wrong at every load except the one it was
tuned for. These four control loops read what completed calls report and answer
one question each: how many calls may be in flight, whether this queued piece of
work is still worth running, and how much of the next retry wave to drop before
it leaves the process.

```ruby
require 'shed_machines'

control = ShedMachines::Vegas.new(limit: 10, min: 1, max: 200)

control.observe(rtt: 0.004, in_flight: 10, outcome: :ok)
control.limit                                              # => 11

control.observe(rtt: 0.040, in_flight: 11, outcome: :ok)
control.limit                                              # => 10
```

Nothing here enforces anything. The caller owns the semaphore, the queue and the
dice.

## The native extension is required

There is no pure-Ruby fallback and no switch to turn one on. `require
'shed_machines'` loads the extension, and fails if it is not there. A control
law with two implementations is a control law with two behaviours, and the whole
point of these four is that they are the same arithmetic everywhere: the gem is
a binding to the `shed-machines` crate in `ext/shed_machines_native/core`, which
is `no_std` with zero dependencies and also runs on bare metal. It needs Ruby
4.0 and a Rust toolchain, or a platform gem.

## Moving a concurrency limit

`Aimd` grows the limit by one whenever a saturated call succeeds, and keeps
three quarters of it on any overload. It needs no latency signal.

```ruby
control = ShedMachines::Aimd.new(limit: 10, min: 1, max: 100)

control.observe(rtt: 0.004, in_flight: 10, outcome: :ok)        # saturated: grows
control.limit                                                   # => 11

control.observe(rtt: 0.004, in_flight: 11, outcome: :overload)
control.limit                                                   # => 8
```

`Vegas` infers queue depth from the gap between the current round-trip time and
the best one it has seen, because latency alone cannot tell a slow dependency
from a deep queue. It grows while fewer than `alpha` calls look queued, shrinks
above `beta`, and holds in between.

```ruby
control = ShedMachines::Vegas.new(limit: 10, min: 1, max: 200, alpha: 3, beta: 6)
control.best_rtt     # => nil, until the first completed call
```

Both take `outcome: :ok` or `outcome: :overload`, where overload means the call
timed out, was rejected, or otherwise says the dependency is loaded. Anything
else raises `ArgumentError`. Both clamp a starting limit into `min..max`, which
default to `LOWEST_FLOOR` and `DEFAULT_CEILING`.

## Shedding by waiting time

Queue length is the wrong signal, since a fast queue can be long and a stalled
one short. `Codel` watches how long work waited and sheds only once that stays
above target for a full interval, so a burst passes through and a standing queue
does not.

```ruby
queue = ShedMachines::Codel.new                 # 5 ms target, 100 ms interval

queue.admit(sojourn: 0.020, now: 0.000)         # => :admit, the burst is tolerated
queue.admit(sojourn: 0.020, now: 0.150)         # => :shed, it is a standing queue
queue.shedding?                                 # => true
queue.admit(sojourn: 0.001, now: 0.200)         # => :admit, and the state clears
```

Successive sheds come faster, as the inverse square root of how many have
happened, so the control law converges instead of oscillating.

## Not amplifying an outage

A client that keeps retrying a failing dependency multiplies the load that broke
it. `Budget` tracks what the client asked for against what was accepted and
reports what fraction of the next wave to drop.

```ruby
budget = ShedMachines::Budget.new               # each accept earns 2 requests

100.times { budget.requested.accepted }
budget.shed_probability                         # => 0.0

300.times { budget.requested }
budget.shed_probability                         # => 0.4987...

budget.decay                                    # halves both counts
```

`shed_probability` is a fraction, not a decision, so nothing here needs a random
number generator.

## Seconds in, nanoseconds inside

Every duration crosses this boundary as seconds in a `Float`, and is converted
once to nanoseconds. The control laws are integer-only on purpose: a limit, a
queue estimate and a backoff are counts, and counting in floats invites a
limiter that is 10.999999 calls wide.

## Constants

The Ruby constants mirror the crate's, with the same names and values.

| Constant                           | Value       |
| ---------------------------------- | ----------- |
| `ShedMachines::LOWEST_FLOOR`       | 1           |
| `ShedMachines::DEFAULT_CEILING`    | 1000        |
| `Aimd::DEFAULT_BACKOFF_KEPT`       | 3           |
| `Aimd::DEFAULT_BACKOFF_OF`         | 4           |
| `Vegas::DEFAULT_ALPHA`             | 3           |
| `Vegas::DEFAULT_BETA`              | 6           |
| `Vegas::OVERLOAD_DIVISOR`          | 2           |
| `Codel::DEFAULT_TARGET_NS`         | 5_000_000   |
| `Codel::DEFAULT_INTERVAL_NS`       | 100_000_000 |
| `Budget::DEFAULT_MULTIPLIER`       | 2.0         |

## License

MIT.
