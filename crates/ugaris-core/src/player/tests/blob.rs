use super::*;

#[test]
fn misc_ppd_blob_preserves_non_tree_legacy_fields() {
    let mut existing_misc = vec![0; LEGACY_MISC_PPD_SIZE];
    write_i32(&mut existing_misc, 0, 123);
    write_i32(&mut existing_misc, 20, 456);
    write_i32(&mut existing_misc, MISC_PPD_GIFT_YEAR_OFFSET, 2024);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_MISC_PPD, &existing_misc);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&existing));
    assert_eq!(
        player.touch_xmas_tree(2, 2025, true, true),
        XmasTreeResult::GiftGranted
    );

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_MISC_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_MISC_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8), 123);
    assert_eq!(read_i32(&encoded, 28), 456);
    assert_eq!(encoded[8 + MISC_PPD_TREEDONE_OFFSET], 0b0000_0100);
    assert_eq!(read_i32(&encoded, 8 + MISC_PPD_GIFT_YEAR_OFFSET), 2025);
}

#[test]
fn malformed_ppd_blob_is_rejected() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut malformed = Vec::new();
    malformed.extend_from_slice(&DRD_KEYRING_PPD.to_le_bytes());
    malformed.extend_from_slice(&(LEGACY_KEYRING_PPD_SIZE as u32).to_le_bytes());
    malformed.extend_from_slice(&[0; 7]);

    assert!(!player.decode_legacy_ppd_blob(&malformed));
}
