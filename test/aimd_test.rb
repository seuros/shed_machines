# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class AimdTest < Minitest::Test
    include Samples

    def control(limit = 10, backoff = Backoff.default)
      Aimd.new(Bounds.new(1, 100), limit, backoff)
    end

    def test_a_success_grows_the_limit_only_while_the_caller_is_saturating_it
      aimd = control

      assert_nil aimd.observe(ok(4 * MS, 9))
      assert_equal 10, aimd.limit

      aimd.observe(ok(4 * MS, 10))

      assert_equal 11, aimd.limit
    end

    def test_an_overload_keeps_three_quarters_of_the_limit
      aimd = control(11)

      aimd.observe(overload(4 * MS, 11))

      assert_equal 8, aimd.limit
    end

    def test_the_limit_stops_at_the_floor_and_the_ceiling
      floor = control(1)
      floor.observe(overload(4 * MS, 1))
      ceiling = control(100)
      ceiling.observe(ok(4 * MS, 100))

      assert_equal [1, 100], [floor.limit, ceiling.limit]
    end

    def test_the_configuration_reads_back_as_the_values_given
      aimd = control(10, Backoff.new(1, 2))

      assert_equal Bounds.new(1, 100), aimd.bounds
      assert_equal Backoff.new(1, 2), aimd.backoff

      aimd.observe(overload(4 * MS, 10))

      assert_equal 5, aimd.limit
    end

    def test_a_starting_limit_outside_the_bounds_is_not_a_controller
      assert_raises(RangeError) { Aimd.new(Bounds.new(2, 10), 1, Backoff.default) }
      assert_raises(RangeError) { Aimd.new(Bounds.new(2, 10), 11, Backoff.default) }
    end

    def test_the_configuration_must_be_the_gems_own_values
      assert_raises(TypeError) { Aimd.new([1, 100], 10, Backoff.default) }
      assert_raises(TypeError) { Aimd.new(Bounds.default, 10, [3, 4]) }
      assert_raises(TypeError) { Aimd.new(Bounds.default, 10.0, Backoff.default) }
      assert_raises(TypeError) { control.observe({ rtt: 1, in_flight: 1, outcome: :ok }) }
    end

    def test_a_frozen_controller_cannot_move
      aimd = control.freeze

      assert_raises(FrozenError) { aimd.observe(ok(4 * MS, 10)) }
      # Frozen is checked before the argument is read.
      assert_raises(FrozenError) { aimd.observe(:not_a_sample) }
      assert_equal 10, aimd.limit
    end
  end
end
