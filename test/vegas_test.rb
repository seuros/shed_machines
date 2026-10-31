# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class VegasTest < Minitest::Test
    MS = 0.001

    # One sample establishes the baseline, and grows the limit from 10 to 11.
    def control(**)
      vegas = Vegas.new(limit: 10, min: 1, max: 100, **)
      vegas.observe(rtt: MS, in_flight: 10, outcome: :ok)
      vegas
    end

    def test_the_first_sample_establishes_the_baseline_and_grows_the_limit
      vegas = Vegas.new(limit: 10, min: 1, max: 100)

      assert_nil vegas.best_rtt

      vegas.observe(rtt: MS, in_flight: 10, outcome: :ok)

      assert_in_delta MS, vegas.best_rtt, 0.0
      assert_equal 11, vegas.limit
    end

    def test_a_slower_call_does_not_move_the_baseline
      vegas = control

      vegas.observe(rtt: 2 * MS, in_flight: 11, outcome: :ok)

      assert_in_delta MS, vegas.best_rtt, 0.0
    end

    def test_fewer_queued_calls_than_alpha_grows_the_limit
      vegas = control

      vegas.observe(rtt: 1.3 * MS, in_flight: 11, outcome: :ok)

      assert_equal 12, vegas.limit
    end

    def test_the_band_holds_the_limit_from_alpha_through_beta
      [1.5 * MS, 2.4 * MS].each do |rtt|
        vegas = control

        vegas.observe(rtt: rtt, in_flight: 11, outcome: :ok)

        assert_equal 11, vegas.limit
      end
    end

    def test_more_queued_calls_than_beta_shrinks_the_limit
      vegas = control

      vegas.observe(rtt: 3 * MS, in_flight: 11, outcome: :ok)

      assert_equal 10, vegas.limit
    end

    def test_the_band_is_configurable
      vegas = control(alpha: 1, beta: 2)

      vegas.observe(rtt: 1.5 * MS, in_flight: 11, outcome: :ok)

      assert_equal 10, vegas.limit
    end

    def test_a_call_with_no_latency_to_report_carries_no_queue_signal
      vegas = Vegas.new(limit: 10, min: 1, max: 100)

      vegas.observe(rtt: 0.0, in_flight: 10, outcome: :ok)

      assert_equal 11, vegas.limit
      assert_in_delta 0.0, vegas.best_rtt, 0.0
    end

    def test_an_overload_halves_the_limit_whatever_the_latency_says
      vegas = control

      vegas.observe(rtt: MS, in_flight: 11, outcome: :overload)

      assert_equal 5, vegas.limit
    end

    def test_an_overload_at_the_floor_leaves_a_limit_to_work_with
      vegas = Vegas.new(limit: 1, min: 1, max: 100)

      vegas.observe(rtt: MS, in_flight: 1, outcome: :overload)

      assert_equal 1, vegas.limit
    end

    def test_an_unknown_outcome_is_rejected
      error = assert_raises(ArgumentError) do
        control.observe(rtt: MS, in_flight: 11, outcome: :timeout)
      end

      assert_equal 'unknown outcome: :timeout', error.message
    end
  end
end
