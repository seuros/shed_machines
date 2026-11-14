# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class CodelTest < Minitest::Test
    TARGET = Codel::DEFAULT_TARGET
    INTERVAL = Codel::DEFAULT_INTERVAL

    def test_the_defaults_are_the_papers_in_integer_nanoseconds
      queue = Codel.default

      assert_equal [5_000_000, 100_000_000], [queue.target, queue.interval]
      queue = Codel.new(TARGET, INTERVAL)

      assert_equal [TARGET, INTERVAL], [queue.target, queue.interval]
    end

    def test_a_standing_queue_is_tolerated_until_the_interval_elapses
      queue = Codel.default

      assert_equal :admit, queue.admit(TARGET, 0)
      assert_equal :admit, queue.admit(TARGET, INTERVAL - 1)
      refute_predicate queue, :shedding?

      assert_equal :shed, queue.admit(TARGET, INTERVAL)
      assert_predicate queue, :shedding?
      assert_equal 1, queue.shed_count
    end

    def test_a_shed_buys_the_queue_a_gap_before_the_next_one
      queue = standing_queue

      assert_equal :admit, queue.admit(TARGET, (2 * INTERVAL) - 1)
      assert_equal :shed, queue.admit(TARGET, 2 * INTERVAL)
      assert_equal 2, queue.shed_count
    end

    def test_dropping_back_under_target_clears_the_shedding_state
      queue = standing_queue

      assert_equal :admit, queue.admit(TARGET - 1, 2 * INTERVAL)

      refute_predicate queue, :shedding?
      assert_equal 0, queue.shed_count
    end

    def test_an_earlier_instant_is_a_refusal_that_changes_nothing
      queue = standing_queue

      assert_equal :time_regression, queue.admit(TARGET, INTERVAL - 1)
      assert_equal :time_regression, queue.admit(TARGET - 1, INTERVAL - 1)
      assert_predicate queue, :shedding?
      assert_equal 1, queue.shed_count
    end

    def test_a_configuration_with_a_zero_span_is_rejected
      assert_raises(RangeError) { Codel.new(0, INTERVAL) }
      assert_raises(RangeError) { Codel.new(TARGET, 0) }
    end

    def test_time_is_integer_nanoseconds
      assert_raises(TypeError) { Codel.new(0.005, INTERVAL) }
      assert_raises(RangeError) { Codel.new(-1, INTERVAL) }
      assert_raises(RangeError) { Codel.new(TARGET, 2**64) }
    end

    # Every argument is converted before the queue is asked, so a bad one
    # raises with the queue exactly as it was.
    def test_a_malformed_argument_raises_before_the_queue_moves
      queue = standing_queue

      assert_raises(TypeError) { queue.admit(TARGET - 1, 2.5) }
      assert_raises(RangeError) { queue.admit(TARGET - 1, -1) }
      assert_predicate queue, :shedding?
      assert_raises(FrozenError) { queue.freeze.admit(TARGET - 1, 2 * INTERVAL) }
      # Frozen is checked before any argument is read.
      assert_raises(FrozenError) { queue.admit(1.5, -1) }
      assert_predicate queue, :shedding?
    end

    private

    # Above target since zero and shedding since the interval elapsed.
    def standing_queue
      queue = Codel.default
      queue.admit(TARGET, 0)
      queue.admit(TARGET, INTERVAL)
      queue
    end
  end
end
