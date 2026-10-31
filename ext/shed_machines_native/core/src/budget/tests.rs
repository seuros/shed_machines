use crate::Budget;

#[test]
fn nothing_is_shed_while_the_dependency_accepts() {
    let mut budget = Budget::default();
    for _ in 0..100 {
        budget.requested();
        budget.accepted();
    }

    assert!((budget.shed_probability() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn the_shed_fraction_climbs_as_the_client_outruns_its_entitlement() {
    const ACCEPTED: u32 = 100;
    const WAVE: u32 = 100;

    let mut budget = Budget::default();
    for _ in 0..ACCEPTED {
        budget.requested();
        budget.accepted();
    }

    // DEFAULT_MULTIPLIER entitles the client to that many waves for free.
    for _ in 0..WAVE {
        budget.requested();
    }
    assert!(budget.shed_probability() < f64::EPSILON);

    let mut previous = 0.0;
    for _ in 0..4 {
        for _ in 0..WAVE {
            budget.requested();
        }
        let shed = budget.shed_probability();
        assert!(shed > previous, "{shed} did not climb past {previous}");
        previous = shed;
    }
    assert!(previous < 1.0);
}

#[test]
fn a_total_outage_never_sheds_everything() {
    let mut budget = Budget::default();
    for _ in 0..1_000_000 {
        budget.requested();
    }

    let shed = budget.shed_probability();
    assert!(shed > 0.99 && shed < 1.0, "{shed}");
}

#[test]
fn decay_lets_the_budget_forgive_a_past_outage() {
    let mut budget = Budget::default();
    for _ in 0..1_000 {
        budget.requested();
    }
    assert!(budget.shed_probability() > 0.9);

    for _ in 0..20 {
        budget.decay();
    }

    assert_eq!(budget.requests(), 0);
    assert!((budget.shed_probability() - 0.0).abs() < f64::EPSILON);
}
