use super::*;

#[test]
fn legacy_random_below_advances_seed_like_c_style_lcg() {
    let mut seed = 0_u32;

    assert_eq!(legacy_random_below_from_seed(&mut seed, 10), 5);
    assert_eq!(seed, 12_345);
    assert_eq!(legacy_random_below_from_seed(&mut seed, 10), 4);
    assert_eq!(legacy_random_below_from_seed(&mut seed, 0), 0);
}
