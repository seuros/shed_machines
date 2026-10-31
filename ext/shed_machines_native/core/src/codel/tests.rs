use crate::{Codel, Verdict};

const MS: u64 = 1_000_000;

fn queue() -> Codel {
    Codel::new(5 * MS, 100 * MS)
}

#[test]
fn work_under_target_is_always_admitted() {
    let mut codel = queue();

    for tick in 0..1_000 {
        assert_eq!(codel.admit(MS, tick * MS), Verdict::Admit);
    }
    assert!(!codel.shedding());
}

#[test]
fn a_burst_shorter_than_the_interval_passes_through() {
    let mut codel = queue();

    assert_eq!(codel.admit(50 * MS, 0), Verdict::Admit);
    assert_eq!(codel.admit(50 * MS, 99 * MS), Verdict::Admit);
    assert!(!codel.shedding());
}

#[test]
fn a_standing_queue_starts_shedding_once_the_interval_elapses() {
    let mut codel = queue();
    codel.admit(50 * MS, 0);

    assert_eq!(codel.admit(50 * MS, 100 * MS), Verdict::Shed);
    assert!(codel.shedding());
}

#[test]
fn dropping_back_under_target_clears_the_shedding_state() {
    let mut codel = queue();
    codel.admit(50 * MS, 0);
    codel.admit(50 * MS, 100 * MS);
    assert!(codel.shedding());

    assert_eq!(codel.admit(MS, 101 * MS), Verdict::Admit);

    assert!(!codel.shedding());
}

#[test]
fn sheds_come_closer_together_the_longer_the_queue_stands() {
    const GAPS: usize = 4;

    let mut codel = queue();
    codel.admit(50 * MS, 0);

    let mut gaps = [0_u64; GAPS];
    let mut found = 0;
    let mut last_shed_at = 0;
    let mut now = 100 * MS;
    while found < GAPS {
        if codel.admit(50 * MS, now) == Verdict::Shed {
            if last_shed_at > 0 {
                gaps[found] = now - last_shed_at;
                found += 1;
            }
            last_shed_at = now;
        }
        now += MS;
    }

    assert!(
        gaps.windows(2).all(|pair| pair[1] <= pair[0]),
        "gaps widened: {gaps:?}"
    );
}
