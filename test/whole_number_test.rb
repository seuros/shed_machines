# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class WholeNumberTest < Minitest::Test
    def test_a_fractional_call_count_is_refused_while_seconds_stay_fractional
      aimd = Aimd.new(limit: 10, min: 1, max: 100)

      assert_raises(TypeError) { Aimd.new(limit: 10.9) }
      assert_raises(TypeError) { aimd.observe(rtt: 0.004, in_flight: 10.9, outcome: :ok) }

      assert_equal 10, aimd.limit

      aimd.observe(rtt: 0.004, in_flight: 10, outcome: :ok)

      assert_equal 11, aimd.limit
      assert_in_delta 0.005, Codel.new(target: 0.005, interval: 0.1).target
    end
  end
end
