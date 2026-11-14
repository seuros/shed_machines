# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class BudgetTest < Minitest::Test
    MAX = (2**64) - 1

    def test_retries_past_the_entitlement_are_shed
      budget = Budget.new(2.0)
      100.times do
        assert_equal :counted, budget.requested
        assert_equal :counted, budget.accepted
      end
      assert_in_delta 0.0, budget.shed_probability, 0.0
      300.times { budget.requested }

      assert_in_delta 0.4987, budget.shed_probability, 0.001
      assert_equal [400, 100], [budget.requests, budget.accepts]
    end

    def test_decay_halves_both_counts
      budget = Budget.restored(2.0, 7, 5)

      assert_nil budget.decay
      assert_equal [3, 2], [budget.requests, budget.accepts]
    end

    def test_a_full_count_refuses_one_more_and_changes_nothing
      budget = Budget.restored(2.0, MAX, MAX - 1)

      assert_equal :overflow, budget.requested
      assert_equal :counted, budget.accepted
      assert_equal :overflow, budget.accepted
      assert_equal [MAX, MAX], [budget.requests, budget.accepts]
    end

    def test_a_multiplier_the_formula_cannot_hold_is_rejected
      [Float::NAN, Float::INFINITY, -2.0, 0.0, 0.999].each do |bad|
        assert_raises(RangeError) { Budget.new(bad) }
        assert_raises(RangeError) { Budget.restored(bad, 0, 0) }
      end
      assert_raises(TypeError) { Budget.new('2.0') }
      assert_raises(TypeError) { Budget.restored(2.0, 1.5, 0) }
      assert_raises(RangeError) { Budget.restored(2.0, -1, 0) }
      assert_in_delta 2.0, Budget::DEFAULT_MULTIPLIER, 0.0
      assert_in_delta 1.0, Budget::LOWEST_MULTIPLIER, 0.0
      assert_in_delta 2.0, Budget.default.multiplier, 0.0
    end

    def test_a_frozen_budget_counts_nothing
      budget = Budget.default.freeze

      assert_raises(FrozenError) { budget.requested }
      assert_raises(FrozenError) { budget.decay }
      assert_equal 0, budget.requests
    end
  end
end
