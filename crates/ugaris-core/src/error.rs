pub const ERR_NONE: i32 = 0;
pub const ERR_ILLEGAL_COORDS: i32 = 1;
pub const ERR_BLOCKED: i32 = 2;
pub const ERR_ILLEGAL_CHARNO: i32 = 3;
pub const ERR_ILLEGAL_DIR: i32 = 4;
pub const ERR_CONFUSED: i32 = 5;
pub const ERR_NO_ITEM: i32 = 6;
pub const ERR_NOT_TAKEABLE: i32 = 7;
pub const ERR_HAVE_CITEM: i32 = 8;
pub const ERR_NO_CITEM: i32 = 9;
pub const ERR_HAVE_ITEM: i32 = 10;
pub const ERR_ILLEGAL_INVPOS: i32 = 11;
pub const ERR_REQUIREMENTS: i32 = 12;
pub const ERR_NO_CHAR: i32 = 13;
pub const ERR_ILLEGAL_ATTACK: i32 = 14;
pub const ERR_ILLEGAL_ITEMNO: i32 = 15;
pub const ERR_DEAD: i32 = 16;
pub const ERR_MANA_LOW: i32 = 17;
pub const ERR_SELF: i32 = 18;
pub const ERR_ILLEGAL_HURT: i32 = 19;
pub const ERR_NOT_VISIBLE: i32 = 20;
pub const ERR_UNCONCIOUS: i32 = 21;
pub const ERR_UNKNOWN_SPELL: i32 = 22;
pub const ERR_NOT_USEABLE: i32 = 23;
pub const ERR_NOT_BODY: i32 = 24;
pub const ERR_UNKNOWN_SKILL: i32 = 25;
pub const ERR_ILLEGAL_POS: i32 = 26;
pub const ERR_NOT_CONTAINER: i32 = 27;
pub const ERR_ALREADY_WORKING: i32 = 28;
pub const ERR_ILLEGAL_STORENO: i32 = 29;
pub const ERR_ILLEGAL_STOREPOS: i32 = 30;
pub const ERR_SOLD_OUT: i32 = 31;
pub const ERR_GOLD_LOW: i32 = 32;
pub const ERR_QUESTITEM: i32 = 33;
pub const ERR_ACCESS_DENIED: i32 = 34;
pub const ERR_NOT_IDLE: i32 = 35;
pub const ERR_NOT_PLAYER: i32 = 36;
pub const ERR_NO_EFFECT: i32 = 37;
pub const ERR_ALREADY_THERE: i32 = 38;

pub const ERROR_OUT_OF_BOUNDS: &str = "Error number out of bounds";

pub const LEGACY_ERROR_STRINGS: [&str; 40] = [
    "Everything's fine, dear",
    "Illegal coordinates",
    "Target is blocked",
    "Illegal character number",
    "Illegal direction",
    "Server is too confused to honor your request",
    "No item present",
    "Item not takeable",
    "There is already an item in citem",
    "No citem present",
    "There is already an item",
    "Illegal inventory position",
    "Requirements not fulfilled",
    "No character present",
    "Illegal attack (the victim is protected)",
    "Illegal item number",
    "Character is unconcious or dead",
    "Not enough mana",
    "Target points to self",
    "Illegal hurt type",
    "Target is not visible",
    "Target is unconcious",
    "Unknown spell",
    "Item not usable",
    "Not a dead body",
    "Unknown skill",
    "Illegal container position",
    "Not a container",
    "Spell already working",
    "Illegal store number",
    "Illegal store position",
    "Item is sold out",
    "Not enough gold",
    "Quest Item",
    "Access denied",
    "Not idle",
    "Not a player character",
    "Would have no effect",
    "Already there",
    ERROR_OUT_OF_BOUNDS,
];

pub fn get_error_string(error: i32) -> &'static str {
    usize::try_from(error)
        .ok()
        .and_then(|index| LEGACY_ERROR_STRINGS.get(index).copied())
        .unwrap_or(ERROR_OUT_OF_BOUNDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_constants_match_legacy_header() {
        assert_eq!(ERR_NONE, 0);
        assert_eq!(ERR_REQUIREMENTS, 12);
        assert_eq!(ERR_ALREADY_WORKING, 28);
        assert_eq!(ERR_ALREADY_THERE, 38);
    }

    #[test]
    fn error_strings_match_legacy_table() {
        assert_eq!(get_error_string(ERR_NONE), "Everything's fine, dear");
        assert_eq!(get_error_string(ERR_BLOCKED), "Target is blocked");
        assert_eq!(
            get_error_string(ERR_DEAD),
            "Character is unconcious or dead"
        );
        assert_eq!(get_error_string(ERR_NOT_USEABLE), "Item not usable");
        assert_eq!(get_error_string(ERR_NOT_BODY), "Not a dead body");
        assert_eq!(get_error_string(ERR_ALREADY_THERE), "Already there");
    }

    #[test]
    fn out_of_range_errors_use_c_fallback_string() {
        assert_eq!(get_error_string(-1), ERROR_OUT_OF_BOUNDS);
        assert_eq!(get_error_string(39), ERROR_OUT_OF_BOUNDS);
        assert_eq!(get_error_string(40), ERROR_OUT_OF_BOUNDS);
    }
}
