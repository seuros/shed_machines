# ShedMachines 🌊

> **Adaptive concurrency limits and load shedding.** The escort that reads the
> convoy's own telemetry and says how wide the lane is, right now.

A fixed bulkhead is a guess that is wrong at every load except the one it was
tuned for. **AMS Jacobson** does not carry a tuned number. These four control
loops read what completed calls report and answer one question each: how many
calls may be in flight, whether this queued piece of work is still worth
running, and how much of the next retry wave to drop before it leaves the
process.

---

## 🚀 Launch Sequence

```bash
# Add to your ship's Gemfile
gem 'shed_machines'
```

Requires **Ruby 4.0+** and a Rust toolchain, or a platform gem.

---

## 🎯 Mission Brief

```ruby
require 'shed_machines'

include ShedMachines
MS = 1_000_000                       # time is Integer nanoseconds

control = Vegas.new(Bounds.new(1, 200), 10, Band.default, Vegas::DEFAULT_BASELINE_WINDOW)

control.observe(Sample.new(4 * MS, 10, :ok))
control.limit                        # => 11

control.observe(Sample.new(40 * MS, 11, :ok))
control.limit                        # => 10
```

The escort advises, it does not fire. Nothing here enforces anything. The
caller owns the semaphore, the queue and the dice.

---

## 🔩 The Native Extension Is Required

There is no pure-Ruby fallback and no switch to turn one on. `require
'shed_machines'` loads the extension, and fails if it is not there. A control
law with two implementations is a control law with two behaviours, and the whole
point of these four is that they are the same arithmetic everywhere: the gem is
a binding to the `shed-machines` crate in `ext/shed_machines_native/core`, which
is `no_std` with zero dependencies and also runs on bare metal. It needs Ruby
4.0 and a Rust toolchain, or a platform gem.

---

## 🎚️ Moving a Concurrency Limit

`Aimd` grows the limit by one whenever a saturated call succeeds, and keeps a
`Backoff` fraction of it on any overload (`Backoff.default` is three quarters).
It needs no latency signal.

```ruby
control = Aimd.new(Bounds.new(1, 100), 10, Backoff.default)

control.observe(Sample.new(4 * MS, 10, :ok))        # saturated: grows
control.limit                                       # => 11

control.observe(Sample.new(4 * MS, 11, :overload))
control.limit                                       # => 8
```

`Vegas` infers queue depth from the gap between the current round-trip time and
the smallest one it has seen recently, because latency alone cannot tell a slow
dependency from a deep queue. It grows while fewer than `alpha` calls look
queued, shrinks above `beta`, and holds in between: that is its `Band`.

```ruby
control = Vegas.new(Bounds.new(1, 200), 10, Band.new(3, 6), 1_000)
control.best_rtt     # => nil, until a call completes without overload
```

That inference only holds when any two calls would take about as long on an idle
dependency. Over a mix whose service times differ by orders of magnitude the
baseline settles on the fastest kind of work, every slower call reads as a deep
queue, and the limit sinks to `min` and stays there — tuning `alpha` and `beta`
cannot fix it, because the signal is measuring the wrong thing. Use `Aimd` for
heterogeneous work, since it steers on outcomes rather than latency, or give
each class of work its own `Vegas`.

The baseline is the smallest round-trip time in the current window of
`baseline_window` samples or the one before it, so one unusually fast call
cannot pin it for the life of the process and a dependency that legitimately
slows down is believed within two windows. `DEFAULT_BASELINE_WINDOW` is 1000;
`control.baseline_window` reads it back.

Both observe a `Sample` whose outcome is `:ok` or `:overload`, where overload
means the call timed out, was rejected, or otherwise says the dependency is
loaded. Both raise `RangeError` for a starting limit outside the `Bounds`.

---

## ⏱️ Shedding by Waiting Time

Queue length is the wrong signal, since a fast queue can be long and a stalled
one short. `Codel` watches how long work waited and sheds only once that stays
above target for a full interval, so a burst passes through and a standing queue
does not.

```ruby
queue = Codel.default                   # 5 ms target, 100 ms interval

queue.admit(20 * MS, 0)                 # => :admit, the burst is tolerated
queue.admit(20 * MS, 150 * MS)          # => :shed, it is a standing queue
queue.shedding?                         # => true
queue.admit(20 * MS, 149 * MS)          # => :time_regression, nothing changed
queue.admit(1 * MS, 200 * MS)           # => :admit, and the state clears
```

`:shed` is not a refusal. "Should this work, which waited this long, still
run?" is a well-formed question and `:shed` is the controller's answer: the call
is honoured, records the instant, counts the shed and schedules the next one,
because that bookkeeping is the control law itself. The one refusal is
`:time_regression`, for a `now` before the latest one an honoured call recorded.

Successive sheds come faster, as the inverse square root of how many have
happened, so the control law converges instead of oscillating.

---

## 🌀 Not Amplifying an Outage

A client that keeps retrying a failing dependency multiplies the load that broke
it. `Budget` tracks what the client asked for against what was accepted and
reports what fraction of the next wave to drop.

```ruby
budget = Budget.new(Budget::DEFAULT_MULTIPLIER)  # each accept earns 2 requests

100.times { budget.requested; budget.accepted }  # each answers :counted
budget.shed_probability                          # => 0.0

300.times { budget.requested }
budget.shed_probability                          # => 0.4987...

budget.decay                                     # halves both counts
Budget.restored(2.0, budget.requests, budget.accepts) # after a restart
```

`shed_probability` is a fraction, not a decision, so nothing here needs a random
number generator.

---

## 📐 Nanoseconds In, Nanoseconds Out

Every instant and span is an `Integer` of nanoseconds, in and out:
`Sample#rtt`, `Codel.new(target, interval)`, `Codel#admit(sojourn, now)`,
`Vegas#best_rtt`. Read the clock with
`Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)`. A `Float` is a
`TypeError`, a negative or 64-bit-overflowing `Integer` a `RangeError`. The
control laws are integer-only on purpose: a limit, a queue estimate and a
backoff are counts, and counting in floats invites a limiter that is 10.999999
calls wide.

---

## 📜 The Escort's Rules

- **Values are valid or do not exist.** `Bounds`, `Sample`, `Backoff` and
  `Band` are `Data` values checked by the core's own constructors when they are
  built, by the gem or by hand: `Bounds.new(0, 10)`, `Sample.new(5, 0, :ok)`,
  `Backoff.new(4, 4)` and `Band.new(7, 6)` raise `RangeError`. A wrong type
  raises `TypeError`. The controllers take them positionally, in the crate's
  order, with no keyword defaults; `.default` is the crate's `Default`. The
  value classes are sealed: subclassing one raises `TypeError` and `allocate`
  is undefined. Sealing stops accidents, not reflection (a named subclass's
  constant is bound before `inherited` raises, and `Marshal.load` skips
  `initialize`), so the binding is the authority: every value it reads must be
  an instance of exactly the gem's class (`TypeError` otherwise, a subclass
  included) and is rebuilt through the core constructor (`RangeError` for a
  forged one), before any controller sees it. The controllers wrap native
  state with no allocator, and are sealed too: subclassing `Aimd`, `Vegas`,
  `Codel` or `Budget` raises `TypeError`.
- **No re-entrant change.** No mutating call answers a Ruby object, and no
  reader holds a controller while it builds its `Data` answer, so a `new`
  that calls back into the controller makes an ordinary call. The binding
  never raises `RuntimeError` for one.
- **A refusal is a Symbol and changes nothing.** `:time_regression` and
  `:overflow` are the only two. Malformed input raises before the core is
  asked, so a raised call changed nothing either.
- **Shedding is an answer.** `:shed` is honoured, and so is `:admit`.
- **No repeats, no scopes.** Nothing here asks for an effect that can already
  hold, and nothing issues a handle. A `Budget` survives a restart through
  `Budget.restored`; the controllers and `Codel` start fresh.

---

## 🧾 Every Answer

| Call | Answers | Records `now` |
| --- | --- | --- |
| `Aimd#observe(sample)`, `Vegas#observe(sample)` | `nil`; never refuses | no clock |
| `Codel#admit(sojourn, now)` | `:admit` for work under target | no: holds whatever the time |
| `Codel#admit(sojourn, now)` | `:admit` or `:shed` for work at or above target | yes |
| `Codel#admit(sojourn, now)` | `:time_regression` | no, nothing changed |
| `Budget#requested`, `Budget#accepted` | `:counted` or `:overflow` (nothing changed) | no clock |
| `Budget#decay` | `nil`; never refuses | no clock |
| `Aimd.new`, `Vegas.new`, `Codel.new`, `Budget.new`, `Budget.restored` | the instance, or `RangeError` for a configuration the core rejects | no clock |
| readers (`limit`, `bounds`, `backoff`, `band`, `baseline_window`, `best_rtt`, `target`, `interval`, `shedding?`, `shed_count`, `multiplier`, `requests`, `accepts`, `shed_probability`) | the value; `best_rtt` is `nil` until a call completes without overload | no clock |

---

## 📊 Constants on the Instrument Panel

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
| `Vegas::DEFAULT_BASELINE_WINDOW`   | 1000        |
| `Codel::DEFAULT_TARGET`            | 5_000_000   |
| `Codel::DEFAULT_INTERVAL`          | 100_000_000 |
| `Budget::DEFAULT_MULTIPLIER`       | 2.0         |
| `Budget::LOWEST_MULTIPLIER`        | 1.0         |

---

## 📡 Credits

**AMS Jacobson**, NCC-2012, *Jacobson*-class escort.

Named for Kathleen Nichols and Van Jacobson, *Controlling Queue Delay* (2012),
the paper that introduced controlled delay, CoDel, and argued that a queue
should be judged by how long work waits in it rather than by how much work is
in it.

---

## License

MIT.
