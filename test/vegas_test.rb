# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class VegasTest < Minitest::Test
    include Samples

    def control(limit = 10, window = Vegas::DEFAULT_BASELINE_WINDOW)
      Vegas.new(Bounds.new(1, 100), limit, Band.default, window)
    end

    def test_the_first_sample_sets_the_baseline
      vegas = control

      assert_nil vegas.best_rtt

      assert_nil vegas.observe(ok(5 * MS, 10))
      assert_equal 5 * MS, vegas.best_rtt
    end

    def test_a_latency_at_the_baseline_grows_the_limit
      vegas = control
      vegas.observe(ok(MS, 10))
      vegas.observe(ok(MS, 11))

      assert_equal 12, vegas.limit
    end

    def test_latency_far_above_the_baseline_shrinks_the_limit
      vegas = control(20)
      vegas.observe(ok(MS, 20))
      grown = vegas.limit

      vegas.observe(ok(50 * MS, grown))

      assert_operator vegas.limit, :<, grown
    end

    def test_an_overload_halves_the_limit_without_touching_the_baseline
      vegas = control(40)
      vegas.observe(ok(MS, 40))
      vegas.observe(overload(9_999 * MS, 41))

      assert_equal 20, vegas.limit
      assert_equal MS, vegas.best_rtt
    end

    def test_a_window_of_one_follows_the_latest_call
      vegas = control(10, 1)
      vegas.observe(ok(MS, 10))
      vegas.observe(ok(9 * MS, 11))

      assert_equal 9 * MS, vegas.best_rtt
    end

    def test_the_configuration_reads_back_as_the_values_given
      vegas = Vegas.new(Bounds.new(1, 100), 10, Band.new(1, 2), 250)

      assert_equal Bounds.new(1, 100), vegas.bounds
      assert_equal Band.new(1, 2), vegas.band
      assert_equal 250, vegas.baseline_window
    end

    def test_a_configuration_the_estimate_cannot_hold_is_not_a_controller
      assert_raises(RangeError) { Vegas.new(Bounds.new(2, 10), 1, Band.default, 8) }
      assert_raises(RangeError) { Vegas.new(Bounds.new(2, 10), 5, Band.default, 0) }
      assert_raises(TypeError) { Vegas.new(Bounds.new(2, 10), 5, Band.default, 8.0) }
      assert_raises(TypeError) { Vegas.new(Bounds.new(2, 10), 5, Bounds.new(3, 6), 8) }
    end
  end
end
