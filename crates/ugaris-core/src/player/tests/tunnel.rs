use super::*;

#[test]
fn tunnel_ppd_used_accessor_defaults_to_zero_and_rejects_negative_levels() {
    let player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.tunnel_used(10), 0);
    assert_eq!(player.tunnel_used(-1), 0);
}

#[test]
fn tunnel_ppd_blob_round_trips_through_encode_decode() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_tunnel_used(90, 5);
    player.set_tunnel_used(200, 10);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut round_tripped = PlayerRuntime::connected(1, 0);
    assert!(round_tripped.decode_legacy_ppd_blob(&encoded));
    assert_eq!(round_tripped.tunnel_ppd, player.tunnel_ppd);
    assert_eq!(round_tripped.tunnel_used(90), 5);
    assert_eq!(round_tripped.tunnel_used(200), 10);
    assert_eq!(round_tripped.tunnel_used(91), 0);
}

#[test]
fn gorwin_ppd_blob_round_trips_through_encode_decode() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_gorwin_tunnel_level(42);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut round_tripped = PlayerRuntime::connected(1, 0);
    assert!(round_tripped.decode_legacy_ppd_blob(&encoded));
    assert_eq!(round_tripped.gorwin_tunnel_level(), 42);
}

#[test]
fn clear_turn_seyan_ppd_clears_tunnel_ppd_but_not_gorwin_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_tunnel_used(50, 3);
    player.set_gorwin_tunnel_level(50);
    assert!(!player.tunnel_ppd.is_empty());

    player.clear_turn_seyan_ppd();
    assert!(player.tunnel_ppd.is_empty());
    assert_eq!(player.tunnel_used(50), 0);
    // C's `turn_seyan` does NOT `del_data` `DRD_GORWIN_PPD` - only
    // `DRD_TUNNEL_PPD` (`tool.c:4362`).
    assert_eq!(player.gorwin_tunnel_level(), 50);
}
