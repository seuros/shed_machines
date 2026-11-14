# frozen_string_literal: true

require_relative 'shed_machines/version'

module ShedMachines
  # The core's `Bounds`: a controller's limit moves within `min..max`. Like
  # `Bounds::new`, it needs 1 <= min <= max: anything else raises RangeError
  # (TypeError for a non-Integer).
  Bounds = Data.define(:min, :max) do
    def self.default = new(LOWEST_FLOOR, DEFAULT_CEILING)

    def initialize(min:, max:)
      min, max = ShedMachines.__send__(:__bounds, min, max)
      super
    end
  end

  # The core's `Sample`: one completed call. `rtt` is its round-trip time in
  # Integer nanoseconds, `in_flight` the calls in flight when it started,
  # including itself, and `outcome` `:ok` or `:overload`. Like `Sample::new`,
  # it needs at least one call in flight: RangeError otherwise, and for an
  # unknown outcome Symbol or a negative rtt; TypeError for a wrong type.
  Sample = Data.define(:rtt, :in_flight, :outcome) do
    def initialize(rtt:, in_flight:, outcome:)
      rtt, in_flight, outcome = ShedMachines.__send__(:__sample, rtt, in_flight, outcome)
      super
    end
  end

  # The core's `Backoff`: `Aimd` keeps `kept / of` of its limit on overload.
  # Like `Backoff::new`, it needs kept < of: RangeError otherwise.
  Backoff = Data.define(:kept, :of) do
    def self.default = new(Aimd::DEFAULT_BACKOFF_KEPT, Aimd::DEFAULT_BACKOFF_OF)

    def initialize(kept:, of:)
      kept, of = ShedMachines.__send__(:__backoff, kept, of)
      super
    end
  end

  # The core's `Band`: `Vegas` grows below `alpha` queued calls and shrinks
  # above `beta`. Like `Band::new`, it needs alpha <= beta: RangeError
  # otherwise.
  Band = Data.define(:alpha, :beta) do
    def self.default = new(Vegas::DEFAULT_ALPHA, Vegas::DEFAULT_BETA)

    def initialize(alpha:, beta:)
      alpha, beta = ShedMachines.__send__(:__band, alpha, beta)
      super
    end
  end
end

# Aimd, Vegas, Codel and Budget are defined by the native extension, which also
# validates the Data values above; there is no pure-Ruby implementation to fall
# back to.
require 'shed_machines_native/shed_machines_native'

module ShedMachines
  private_class_method :__bounds, :__sample, :__backoff, :__band

  # The values are final. `initialize` is where each validates, and a subclass
  # could override it, or `allocate` build an instance that never ran it:
  # subclassing raises TypeError (as subclassing Class does) and `allocate`
  # is gone. `new`, keywords, `with` and `dup` all still run the check.
  [Bounds, Sample, Backoff, Band].each do |value|
    value.singleton_class.undef_method(:allocate)
    value.define_singleton_method(:inherited) { |_| raise TypeError, "#{value} cannot be subclassed" }
  end
end
