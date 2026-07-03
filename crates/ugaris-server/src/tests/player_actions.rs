use super::*;

#[test]
fn client_center_map_position_matches_legacy_cmap_index() {
    assert_eq!(client_center_map_position(25), 25 + 25 * 51);
    assert_eq!(client_center_map_position(40), 40 + 40 * 81);
}
