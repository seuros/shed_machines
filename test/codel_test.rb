# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class CodelTest < Minitest::Test
    TARGET = Codel::DEFAULT_TARGET_NS / NS_PER_SECOND.to_f
    INTERVAL = Codel::DEFAULT_INTERVAL_NS / NS_PER_SECOND.to_f
    TICK = 1 / NS_PER_SECOND.to_f

    def test_work_under_target_is_always_admitted
      queue = Codel.new

      100.times do |tick|
        assert_equal :admit, queue.admit(sojourn: TARGET - TICK, now: tick * INTERVAL)
      end

      refute_predicate queue, :shedding?
    end

    def test_a_standing_queue_is_tolerated_until_the_interval_elapses
      queue = Codel.new

      assert_equal :admit, queue.admit(sojourn: TARGET, now: 0.0)
      assert_equal :admit, queue.admit(sojourn: TARGET, now: INTERVAL - TICK)
      refute_predicate queue, :shedding?

      assert_equal :shed, queue.admit(sojourn: TARGET, now: INTERVAL)
      assert_predicate queue, :shedding?
    end

    def test_a_shed_buys_the_queue_a_gap_before_the_next_one
      queue = standing_queue

      assert_equal :admit, queue.admit(sojourn: TARGET, now: (2 * INTERVAL) - TICK)
      assert_equal :shed, queue.admit(sojourn: TARGET, now: 2 * INTERVAL)
    end

    def test_dropping_back_under_target_clears_the_shedding_state
      queue = standing_queue

      assert_equal :admit, queue.admit(sojourn: TARGET - TICK, now: 2 * INTERVAL)

      refute_predicate queue, :shedding?
      assert_equal :admit, queue.admit(sojourn: TARGET, now: 2 * INTERVAL)
    end

    def test_the_target_and_interval_are_configurable
      queue = Codel.new(target: 0.02, interval: 0.5)

      assert_in_delta 0.02, queue.target, 0.0
      assert_equal :admit, queue.admit(sojourn: 0.019, now: 0.0)
      assert_equal :admit, queue.admit(sojourn: 0.02, now: 0.0)
      assert_equal :admit, queue.admit(sojourn: 0.02, now: 0.49)
      assert_equal :shed, queue.admit(sojourn: 0.02, now: 0.5)
    end

    def test_a_sojourn_the_arithmetic_cannot_hold_is_rejected
      assert_raises(ArgumentError) { Codel.new.admit(sojourn: -1.0, now: 0.0) }
      assert_raises(ArgumentError) { Codel.new.admit(sojourn: 0.0, now: Float::INFINITY) }
    end

    private

    # Above target since zero and shedding since the interval elapsed.
    def standing_queue
      queue = Codel.new
      queue.admit(sojourn: TARGET, now: 0.0)
      queue.admit(sojourn: TARGET, now: INTERVAL)
      queue
    end
  end
end
