use super::*;

#[test]
fn client_center_map_position_matches_legacy_cmap_index() {
    assert_eq!(client_center_map_position(25), 25 + 25 * 51);
    assert_eq!(client_center_map_position(40), 40 + 40 * 81);
}

#[test]
fn format_client_log_message_matches_legacy_charlog_shape() {
    // C `charlog` (`src/system/logging/log.c`): "<name> (<cn>): <message>
    // [ID=<charID><,IP=...>]"; we omit the optional IP suffix.
    assert_eq!(
        format_client_log_message("Godmode", 42, "client crash: null texture at slot 3"),
        "Godmode (42): client crash: null texture at slot 3 [ID=42]"
    );
}

#[test]
fn apply_player_action_ignores_nop_client_info_log_and_mod_packet() {
    let mut player = PlayerRuntime::connected(1, 0);
    let before = format!("{player:?}");
    let characters = HashMap::new();
    for action in [
        ClientAction::Nop,
        ClientAction::ClientInfo(vec![1, 2, 3]),
        ClientAction::Log(b"hello".to_vec()),
        ClientAction::ModPacket {
            packet_type: 58,
            subtype: 0x01,
            bytes: vec![58, 3, 0x01],
        },
    ] {
        apply_player_action(&mut player, &action, 0, &characters);
        assert_eq!(
            format!("{player:?}"),
            before,
            "action {action:?} should not mutate PlayerRuntime"
        );
        assert_eq!(
            action_to_queued(&action),
            None,
            "action {action:?} should not queue a driver action"
        );
    }
}
