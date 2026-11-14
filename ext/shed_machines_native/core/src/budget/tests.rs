use crate::{Budget, Refusal};
use matryona_machines::testing::{XorShift64, refuses};

#[test]
fn the_shed_fraction_climbs_as_the_client_outruns_its_entitlement() {
    const ACCEPTED: u32 = 100;
    const WAVE: u32 = 100;

    let mut budget = Budget::default();
    for _ in 0..ACCEPTED {
        assert_eq!(budget.requested(), Ok(()));
        assert_eq!(budget.accepted(), Ok(()));
    }
    assert!(budget.shed_probability() < f64::EPSILON);

    // DEFAULT_MULTIPLIER entitles the client to that many waves for free.
    for _ in 0..WAVE {
        assert_eq!(budget.requested(), Ok(()));
    }
    assert!(budget.shed_probability() < f64::EPSILON);

    let mut previous = 0.0;
    for _ in 0..4 {
        for _ in 0..WAVE {
            assert_eq!(budget.requested(), Ok(()));
        }
        let shed = budget.shed_probability();
        assert!(shed > previous, "{shed} did not climb past {previous}");
        previous = shed;
    }
    assert!(previous < 1.0);
}

#[test]
fn a_total_outage_never_sheds_everything() {
    let budget = restored(1_000_000, 0);

    let shed = budget.shed_probability();
    assert!(shed > 0.99 && shed < 1.0, "{shed}");
}

#[test]
fn decay_lets_the_budget_forgive_a_past_outage() {
    let mut budget = restored(1_000, 0);
    assert!(budget.shed_probability() > 0.9);

    for _ in 0..20 {
        budget.decay();
    }

    assert_eq!(budget.requests(), 0);
    assert!((budget.shed_probability() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn a_multiplier_the_formula_cannot_hold_is_not_a_budget() {
    for multiplier in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -2.0, 0.0, 0.999] {
        assert_eq!(Budget::new(multiplier), None, "{multiplier}");
        assert_eq!(Budget::restored(multiplier, 1, 1), None, "{multiplier}");
    }
    assert_eq!(Budget::new(2.0), Some(Budget::default()));
    assert_eq!(
        Budget::new(1.0).map(|budget| budget.multiplier().to_bits()),
        Some(1.0_f64.to_bits())
    );
}

#[test]
fn a_full_count_refuses_one_more_and_changes_nothing() {
    let mut budget = restored(u64::MAX, u64::MAX - 1);

    refuses(&mut budget, Refusal::Overflow, Budget::requested);

    assert_eq!(budget.accepted(), Ok(()));
    refuses(&mut budget, Refusal::Overflow, Budget::accepted);
    assert_eq!((budget.requests(), budget.accepts()), (u64::MAX, u64::MAX));
}

/// Counts near the top of their range, so overflow is reached as often as not.
/// One counting call on a budget.
type Count = fn(&mut Budget) -> Result<(), Refusal>;

#[test]
fn the_counts_follow_a_reference_model() {
    let mut dice = XorShift64::new(11_589_351);
    for _ in 0..100 {
        let start = (
            u64::MAX - dice.next_u64() % 8,
            u64::MAX - dice.next_u64() % 8,
        );
        let mut budget = restored(start.0, start.1);
        let mut model = (u128::from(start.0), u128::from(start.1));
        for _ in 0..64 {
            let (call, expected): (Count, _) = match dice.next_u64() % 5 {
                0 | 1 => (Budget::requested, bump(&mut model.0)),
                2 | 3 => (Budget::accepted, bump(&mut model.1)),
                _ => {
                    model = (model.0 / 2, model.1 / 2);
                    (
                        |b: &mut Budget| {
                            b.decay();
                            Ok(())
                        },
                        Ok(()),
                    )
                }
            };
            match expected {
                Err(refusal) => refuses(&mut budget, refusal, call),
                Ok(()) => assert_eq!(call(&mut budget), Ok(())),
            }
            assert_eq!(
                (u128::from(budget.requests()), u128::from(budget.accepts())),
                model
            );
        }
    }
}

/// A budget at the default multiplier carrying these counts.
fn restored(requests: u64, accepts: u64) -> Budget {
    Budget::restored(2.0, requests, accepts).expect("2.0 is a multiplier")
}

fn bump(count: &mut u128) -> Result<(), Refusal> {
    if *count == u128::from(u64::MAX) {
        return Err(Refusal::Overflow);
    }
    *count += 1;
    Ok(())
}
