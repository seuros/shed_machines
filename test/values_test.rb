# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  # Every Data value is held to its core constructor's rule, whether the gem
  # returned it or it was built by hand.
  class ValuesTest < Minitest::Test
    def test_bounds_need_a_floor_of_one_and_a_ceiling_above_it
      assert_raises(RangeError) { Bounds.new(0, 10) }
      assert_raises(RangeError) { Bounds.new(10, 9) }
      assert_equal [1, 1], Bounds.new(1, 1).deconstruct
      assert_equal Bounds.new(LOWEST_FLOOR, DEFAULT_CEILING), Bounds.default
    end

    def test_a_sample_needs_a_call_in_flight_and_a_known_outcome
      assert_raises(RangeError) { Sample.new(0, 0, :ok) }
      assert_raises(RangeError) { Sample.new(0, 1, :degraded) }
      assert_raises(TypeError) { Sample.new(0, 1, 'ok') }

      sample = Sample.new(7, 3, :overload)

      assert_equal [7, 3, :overload], [sample.rtt, sample.in_flight, sample.outcome]
      assert_equal sample, Sample.new(rtt: 7, in_flight: 3, outcome: :overload)
    end

    def test_a_backoff_is_a_fraction_below_one
      assert_raises(RangeError) { Backoff.new(4, 4) }
      assert_raises(RangeError) { Backoff.new(1, 0) }
      assert_equal Backoff.new(3, 4), Backoff.default
      assert_equal [0, 1], Backoff.new(0, 1).deconstruct
    end

    def test_a_band_grows_below_where_it_shrinks
      assert_raises(RangeError) { Band.new(4, 3) }
      assert_equal Band.new(3, 6), Band.default
      assert_equal [3, 3], Band.new(3, 3).deconstruct
    end

    def test_with_is_held_to_the_same_rule
      assert_raises(RangeError) { Bounds.default.with(min: 0) }
      assert_raises(RangeError) { Band.default.with(alpha: 7) }
    end

    # Counts and nanoseconds are Integers: a Float is a TypeError, not a
    # truncation, and an Integer out of range is a RangeError.
    def test_counts_and_nanoseconds_are_integers_in_range
      assert_raises(TypeError) { Bounds.new(1.9, 10) }
      assert_raises(TypeError) { Sample.new(1, 10.9, :ok) }
      assert_raises(TypeError) { Sample.new(0.004, 10, :ok) }
      assert_raises(TypeError) { Backoff.new(nil, 4) }
      assert_raises(RangeError) { Bounds.new(1, 2**32) }
      assert_raises(RangeError) { Sample.new(-1, 1, :ok) }
      assert_raises(RangeError) { Sample.new(2**64, 1, :ok) }
    end

    # A subclass overriding `initialize` without `super`, or `allocate`, would
    # build a value that never ran the check.
    def test_values_are_final
      [Bounds, Sample, Backoff, Band].each do |value|
        assert_raises(TypeError) { Class.new(value) { def initialize(**) = nil } }
        assert_raises(NoMethodError) { value.allocate }
      end
    end

    def test_the_validation_hooks_are_private
      assert_raises(NoMethodError) { ShedMachines.__bounds(1, 2) }
      assert_raises(NoMethodError) { ShedMachines.__sample(1, 1, :ok) }
    end
  end
end
