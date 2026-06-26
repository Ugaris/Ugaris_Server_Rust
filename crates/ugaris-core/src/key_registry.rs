const fn make_item_id(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

const DEV_ID_DB: u32 = 0x01;
const DEV_ID_MR: u32 = 0x02;
const DEV_ID_WARR: u32 = 0x06;
const DEV_ID_RHORUN: u32 = 0x07;
const DEV_ID_WHITESTAR: u32 = 0x08;
const DEV_ID_VELVET: u32 = 0x0A;
const DEV_ID_MAX: u32 = 0x0E;
const DEV_ID_FIRENA: u32 = 0x13;

pub const REGISTERED_KEY_IDS: &[u32] = &[
    make_item_id(DEV_ID_DB, 0x000002),
    make_item_id(DEV_ID_DB, 0x000003),
    make_item_id(DEV_ID_DB, 0x000004),
    make_item_id(DEV_ID_DB, 0x000006),
    make_item_id(DEV_ID_DB, 0x00000A),
    make_item_id(DEV_ID_DB, 0x00000C),
    make_item_id(DEV_ID_DB, 0x00000E),
    make_item_id(DEV_ID_DB, 0x00000F),
    make_item_id(DEV_ID_DB, 0x000010),
    make_item_id(DEV_ID_DB, 0x000011),
    make_item_id(DEV_ID_DB, 0x000012),
    make_item_id(DEV_ID_DB, 0x000013),
    make_item_id(DEV_ID_DB, 0x000014),
    make_item_id(DEV_ID_DB, 0x000015),
    make_item_id(DEV_ID_DB, 0x000016),
    make_item_id(DEV_ID_DB, 0x000017),
    make_item_id(DEV_ID_DB, 0x000018),
    make_item_id(DEV_ID_DB, 0x000019),
    make_item_id(DEV_ID_DB, 0x00001A),
    make_item_id(DEV_ID_DB, 0x00001B),
    make_item_id(DEV_ID_DB, 0x00001E),
    make_item_id(DEV_ID_DB, 0x00001F),
    make_item_id(DEV_ID_DB, 0x000020),
    make_item_id(DEV_ID_DB, 0x000021),
    make_item_id(DEV_ID_DB, 0x000023),
    make_item_id(DEV_ID_DB, 0x000024),
    make_item_id(DEV_ID_DB, 0x00002D),
    make_item_id(DEV_ID_DB, 0x000039),
    make_item_id(DEV_ID_DB, 0x000030),
    make_item_id(DEV_ID_DB, 0x000031),
    make_item_id(DEV_ID_DB, 0x000032),
    make_item_id(DEV_ID_DB, 0x000033),
    make_item_id(DEV_ID_DB, 0x000029),
    make_item_id(DEV_ID_DB, 0x00002A),
    make_item_id(DEV_ID_DB, 0x00002B),
    make_item_id(DEV_ID_DB, 0x00002E),
    make_item_id(DEV_ID_DB, 0x000044),
    make_item_id(DEV_ID_DB, 0x000045),
    make_item_id(DEV_ID_DB, 0x000046),
    make_item_id(DEV_ID_DB, 0x000047),
    make_item_id(DEV_ID_DB, 0x00004F),
    make_item_id(DEV_ID_DB, 0x000050),
    make_item_id(DEV_ID_DB, 0x000052),
    make_item_id(DEV_ID_DB, 0x000053),
    make_item_id(DEV_ID_DB, 0x000054),
    make_item_id(DEV_ID_DB, 0x000055),
    make_item_id(DEV_ID_DB, 0x000056),
    make_item_id(DEV_ID_DB, 0x000057),
    make_item_id(DEV_ID_DB, 0x000058),
    make_item_id(DEV_ID_DB, 0x000059),
    make_item_id(DEV_ID_DB, 0x000070),
    make_item_id(DEV_ID_DB, 0x000060),
    make_item_id(DEV_ID_DB, 0x000061),
    make_item_id(DEV_ID_DB, 0x000063),
    make_item_id(DEV_ID_DB, 0x000064),
    make_item_id(DEV_ID_DB, 0x000065),
    make_item_id(DEV_ID_DB, 0x00006C),
    make_item_id(DEV_ID_DB, 0x00006D),
    make_item_id(DEV_ID_DB, 0x000072),
    make_item_id(DEV_ID_DB, 0x000073),
    make_item_id(DEV_ID_DB, 0x000076),
    make_item_id(DEV_ID_DB, 0x000081),
    make_item_id(DEV_ID_DB, 0x000082),
    make_item_id(DEV_ID_DB, 0x000083),
    make_item_id(DEV_ID_DB, 0x000084),
    make_item_id(DEV_ID_DB, 0x000085),
    make_item_id(DEV_ID_DB, 0x000086),
    make_item_id(DEV_ID_DB, 0x000087),
    make_item_id(DEV_ID_DB, 0x000088),
    make_item_id(DEV_ID_DB, 0x000089),
    make_item_id(DEV_ID_DB, 0x00008E),
    make_item_id(DEV_ID_DB, 0x000090),
    make_item_id(DEV_ID_DB, 0x000091),
    make_item_id(DEV_ID_DB, 0x0000B0),
    make_item_id(DEV_ID_DB, 0x0000B1),
    make_item_id(DEV_ID_DB, 0x0000B2),
    make_item_id(DEV_ID_DB, 0x0000B3),
    make_item_id(DEV_ID_VELVET, 0x000014),
    make_item_id(DEV_ID_DB, 0x0000CA),
    make_item_id(DEV_ID_DB, 0x0000CB),
    make_item_id(DEV_ID_DB, 0x0000CC),
    make_item_id(DEV_ID_DB, 0x0000CD),
    make_item_id(DEV_ID_DB, 0x0000CE),
    make_item_id(DEV_ID_FIRENA, 0x000014),
    make_item_id(DEV_ID_FIRENA, 0x000089),
    make_item_id(DEV_ID_MR, 0x000006),
    make_item_id(DEV_ID_MR, 0x000007),
    make_item_id(DEV_ID_MR, 0x000008),
    make_item_id(DEV_ID_MR, 0x000011),
    make_item_id(DEV_ID_WARR, 0x000001),
    make_item_id(DEV_ID_WARR, 0x000002),
    make_item_id(DEV_ID_WARR, 0x000003),
    make_item_id(DEV_ID_WARR, 0x000004),
    make_item_id(DEV_ID_WARR, 0x00000A),
    make_item_id(DEV_ID_WARR, 0x00000B),
    make_item_id(DEV_ID_WARR, 0x00000C),
    make_item_id(DEV_ID_WARR, 0x00000D),
    make_item_id(DEV_ID_WARR, 0x00000E),
    make_item_id(DEV_ID_WARR, 0x00000F),
    make_item_id(DEV_ID_WARR, 0x000010),
    make_item_id(DEV_ID_WARR, 0x000011),
    make_item_id(DEV_ID_WARR, 0x000012),
    make_item_id(DEV_ID_WARR, 0x000013),
    make_item_id(DEV_ID_WARR, 0x000014),
    make_item_id(DEV_ID_WARR, 0x000015),
    make_item_id(DEV_ID_WARR, 0x000016),
    make_item_id(DEV_ID_WARR, 0x000017),
    make_item_id(DEV_ID_WARR, 0x000018),
    make_item_id(DEV_ID_WARR, 0x000019),
    make_item_id(DEV_ID_WARR, 0x00001A),
    make_item_id(DEV_ID_WARR, 0x00001B),
    make_item_id(DEV_ID_WARR, 0x00001C),
    make_item_id(DEV_ID_WARR, 0x00001D),
    make_item_id(DEV_ID_WARR, 0x00001E),
    make_item_id(DEV_ID_WARR, 0x000022),
    make_item_id(DEV_ID_WARR, 0x000023),
    make_item_id(DEV_ID_WARR, 0x000024),
    make_item_id(DEV_ID_WARR, 0x00002A),
    make_item_id(DEV_ID_WARR, 0x00002B),
    make_item_id(DEV_ID_WARR, 0x00002C),
    make_item_id(DEV_ID_WARR, 0x00002D),
    make_item_id(DEV_ID_RHORUN, 0x000001),
    make_item_id(DEV_ID_RHORUN, 0x000002),
    make_item_id(DEV_ID_RHORUN, 0x000003),
    make_item_id(DEV_ID_RHORUN, 0x000004),
    make_item_id(DEV_ID_WHITESTAR, 0x000001),
    make_item_id(DEV_ID_WHITESTAR, 0x000002),
    make_item_id(DEV_ID_WHITESTAR, 0x000003),
    make_item_id(DEV_ID_WHITESTAR, 0x000004),
    make_item_id(DEV_ID_WHITESTAR, 0x000005),
    make_item_id(DEV_ID_WHITESTAR, 0x000006),
    make_item_id(DEV_ID_WHITESTAR, 0x000007),
    make_item_id(DEV_ID_WHITESTAR, 0x000008),
    make_item_id(DEV_ID_WHITESTAR, 0x000009),
    make_item_id(DEV_ID_WHITESTAR, 0x00000A),
    make_item_id(DEV_ID_WHITESTAR, 0x00000E),
    make_item_id(DEV_ID_MAX, 0x000001),
];

pub fn is_registered_key(id: u32) -> bool {
    REGISTERED_KEY_IDS.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_key_list_matches_legacy_count_and_known_ids() {
        assert_eq!(REGISTERED_KEY_IDS.len(), 137);
        assert!(is_registered_key(make_item_id(DEV_ID_DB, 0x000002)));
        assert!(is_registered_key(make_item_id(DEV_ID_DB, 0x000070)));
        assert!(is_registered_key(make_item_id(DEV_ID_WHITESTAR, 0x00000E)));
        assert!(!is_registered_key(make_item_id(DEV_ID_DB, 0x000001)));
        assert!(!is_registered_key((59 << 24) | 0x000002));
    }
}
