use super::*;

pub(crate) const ARKHATA_CLERK_TIME_SECONDS: i32 = 60 * 15;

pub(crate) const DRD_ACCOUNT_WIDE_DEPOT: u32 =
    (ugaris_core::player::DEV_ID_ED << 24) | (6 | ugaris_core::player::PERSISTENT_SUBSCRIBER_DATA);

/// C `#define DRD_ACHIEVEMENT_DATA MAKE_DRD(DEV_ID_ED, 11 | PERSISTENT_SUBSCRIBER_DATA)`
/// (`src/system/drdata.h:266`).
pub(crate) const DRD_ACHIEVEMENT_DATA: u32 =
    (ugaris_core::player::DEV_ID_ED << 24) | (11 | ugaris_core::player::PERSISTENT_SUBSCRIBER_DATA);

/// C `#define DRD_ACHIEVEMENT_STATS MAKE_DRD(DEV_ID_ED, 12 | PERSISTENT_SUBSCRIBER_DATA)`
/// (`src/system/drdata.h:267`).
pub(crate) const DRD_ACHIEVEMENT_STATS: u32 =
    (ugaris_core::player::DEV_ID_ED << 24) | (12 | ugaris_core::player::PERSISTENT_SUBSCRIBER_DATA);

pub(crate) const LEGACY_CONTAINER_SIZE: usize = ugaris_core::entity::INVENTORY_SIZE - 2;

pub(crate) const IID_AREA19_WOLFSSKIN: u32 = 0x0100008A;

pub(crate) const IID_AREA19_SALT: u32 = 0x0100008B;

pub(crate) const IID_AREA19_WOLFSSKIN2: u32 = 0x0100008C;

pub(crate) const IID_BRONZECHIP: u32 = 0x010000AC;

pub(crate) const IID_SILVERCHIP: u32 = 0x010000AD;

pub(crate) const IID_GOLDCHIP: u32 = 0x010000AE;

pub(crate) const LOGIN_SPAWN_X: usize = 128;

pub(crate) const LOGIN_SPAWN_Y: usize = 128;

pub(crate) const LOGIN_ACCEPTED_MESSAGE: &str = "Rust Ugaris compatibility login accepted.";

// C `read_login` (`src/system/player.c:396-444`): the exact reject text sent
// via `player_client_exit`/`SV_EXIT` for each `find_login` error code. Text
// must match digit-for-digit since it is rendered verbatim by the legacy
// client.
pub(crate) const LOGIN_REJECT_INTERNAL_ERROR: &str =
    "Internal error. Please try again. If several retries fail email game@ugaris.com.";
pub(crate) const LOGIN_REJECT_LOCKED: &str =
    "You have been banned. Please email game@ugaris.com for details.";
pub(crate) const LOGIN_REJECT_WRONG_PASSWORD: &str = "Username or password wrong.";
pub(crate) const LOGIN_REJECT_DUPLICATE: &str =
    "Duplicate login. Please make sure no other character from your account is active.";
pub(crate) const LOGIN_REJECT_NOT_PAID: &str = "Your account has not been paid.";
pub(crate) const LOGIN_REJECT_SHUTDOWN: &str =
    "The server is being shut down. Please try again in a few minutes.";
pub(crate) const LOGIN_REJECT_IP_LOCKED: &str = "Your IP address is banned. Please email game@ugaris.com with your account ID and ask for an exception to be made.";
pub(crate) const LOGIN_REJECT_ACCOUNT_NOT_FIXED: &str = "Please log onto your account management on www.ugaris.com and update the account ownership information. Scroll down to 'Address Information' and choose 'Edit'.";
pub(crate) const LOGIN_REJECT_TOO_MANY_BAD_PASSWORDS: &str =
    "Too many tries with bad passwords. Please come back later.";

pub(crate) const CHEST_EMPTY_MESSAGE: &str = "The chest is empty.";

pub(crate) const CHEST_CURSOR_OCCUPIED_MESSAGE: &str =
    "Please empty your 'hand' (mouse cursor) first.";

pub(crate) const CHEST_KEY_REQUIRED_MESSAGE: &str = "You need a key to open this chest.";

pub(crate) const RANDCHEST_CURSOR_OCCUPIED_MESSAGE: &str =
    "Please empty your hand (mouse cursor) first.";

pub(crate) const RANDCHEST_EMPTY_MESSAGE: &str = "You didn't find anything.";

pub(crate) const TORCH_UNDERWATER_MESSAGE: &str =
    "Obviously, thou canst not light thy torch under water.";

pub(crate) const TORCH_HISS_MESSAGE: &str = "Your hear your torch hiss.";

pub(crate) const MAP_BOOTSTRAP_CHUNK_TARGET: usize = MAX_LEGACY_TICK_PAYLOAD - 512;

pub(crate) const MAX_CLIENT_EFFECTS: usize = 64;

pub(crate) const DEFAULT_PLAYER_TEMPLATE: &str = "seyan_m";

pub(crate) const IID_KEY_RING: u32 = (59 << 24) | 0x000002;
