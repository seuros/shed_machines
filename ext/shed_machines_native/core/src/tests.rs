use super::Bounds;

#[test]
fn bounds_clamp_both_ways() {
    let bounds = Bounds::new(2, 10);

    assert_eq!(bounds.clamp(0), 2);
    assert_eq!(bounds.clamp(5), 5);
    assert_eq!(bounds.clamp(99), 10);
}
