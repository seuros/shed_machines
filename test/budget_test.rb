# frozen_string_literal: true

require 'test_helper'

module ShedMachines
  class BudgetTest < Minitest::Test
    def test_a_client_with_no_history_sheds_nothing
      assert_in_delta 0.0, Budget.new.shed_probability, 0.0
    end

    def test_nothing_is_shed_while_the_client_stays_inside_what_it_earned
      budget = Budget.new
      10.times { budget.requested.accepted }
      10.times { budget.requested }

      assert_equal 20, budget.requests
      assert_equal 10, budget.accepts
      assert_in_delta 0.0, budget.shed_probability, 0.0
    end

    def test_one_request_past_the_entitlement_starts_shedding
      budget = Budget.new
      10.times { budget.requested.accepted }
      11.times { budget.requested }

      assert_in_delta 1.0 / 22, budget.shed_probability, Float::EPSILON
    end

    def test_a_client_talking_to_a_dead_dependency_sheds_nearly_everything
      budget = Budget.new
      1_000.times { budget.requested }

      assert_operator budget.shed_probability, :>, 0.99
    end

    def test_the_multiplier_sets_how_much_each_accepted_request_earns
      budget = Budget.new(multiplier: 1.0)
      10.times { budget.requested.accepted }
      budget.requested

      assert_in_delta 1.0 / 12, budget.shed_probability, Float::EPSILON
    end

    def test_decay_halves_both_counts_and_drops_the_odd_one
      budget = Budget.new
      7.times { budget.requested }
      5.times { budget.accepted }

      budget.decay

      assert_equal 3, budget.requests
      assert_equal 2, budget.accepts
    end
  end
end
