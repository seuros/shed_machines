# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class AimdTest < Minitest::Test
    def control(limit: 10, **)
      Aimd.new(limit: limit, min: 1, max: 100, **)
    end

    def test_a_success_grows_the_limit_only_while_the_caller_is_saturating_it
      aimd = control

      aimd.observe(rtt: 0.004, in_flight: 9, outcome: :ok)

      assert_equal 10, aimd.limit

      aimd.observe(rtt: 0.004, in_flight: 10, outcome: :ok)

      assert_equal 11, aimd.limit
    end

    def test_an_overload_keeps_three_quarters_of_the_limit
      aimd = control(limit: 11)

      aimd.observe(rtt: 0.004, in_flight: 11, outcome: :overload)

      assert_equal 8, aimd.limit
    end

    def test_an_overload_at_the_floor_leaves_a_limit_to_work_with
      aimd = control(limit: 1)

      aimd.observe(rtt: 0.004, in_flight: 1, outcome: :overload)

      assert_equal 1, aimd.limit
    end

    def test_the_limit_stops_at_the_ceiling
      aimd = control(limit: 100)

      aimd.observe(rtt: 0.004, in_flight: 100, outcome: :ok)

      assert_equal 100, aimd.limit
    end

    def test_a_starting_limit_outside_the_bounds_is_pulled_inside_them
      assert_equal LOWEST_FLOOR, Aimd.new(limit: 0).limit
      assert_equal DEFAULT_CEILING, Aimd.new(limit: DEFAULT_CEILING + 1).limit
    end

    def test_the_backoff_fraction_is_configurable
      aimd = control(limit: 10, kept: 1, of: 2)

      aimd.observe(rtt: 0.004, in_flight: 10, outcome: :overload)

      assert_equal 5, aimd.limit
    end

    def test_an_unknown_outcome_is_rejected
      error = assert_raises(ArgumentError) do
        control.observe(rtt: 0.004, in_flight: 10, outcome: :degraded)
      end

      assert_equal 'unknown outcome: :degraded', error.message
    end

    def test_a_starting_limit_is_required
      assert_raises(ArgumentError) { Aimd.new(min: 1, max: 100) }
    end
  end
end
