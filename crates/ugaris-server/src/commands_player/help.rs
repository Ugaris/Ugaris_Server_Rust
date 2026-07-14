use super::*;

pub(crate) fn apply_time_command(date: GameDate, command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 2 || !"time".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![format!(
        "It's {:02}:{:02} on the {}/{}/{}. Sunrise is at {:02}:{:02}, sunset at {:02}:{:02}. Moonrise is at {:02}:{:02}, moonset at {:02}:{:02}.",
        date.hour,
        date.minute,
        date.month + 1,
        date.mday + 1,
        date.year,
        date.sunrise / HOUR_LEN,
        (date.sunrise % HOUR_LEN) / MIN_LEN,
        date.sunset / HOUR_LEN,
        (date.sunset % HOUR_LEN) / MIN_LEN,
        date.moonrise / HOUR_LEN,
        (date.moonrise % HOUR_LEN) / MIN_LEN,
        date.moonset / HOUR_LEN,
        (date.moonset % HOUR_LEN) / MIN_LEN,
    )];

    if !date.fullmoon && !date.newmoon {
        if date.moonsize < 3 {
            messages.push("Quarter Moon.".to_string());
        } else if date.moonsize < 10 {
            messages.push("Half Moon.".to_string());
        } else {
            messages.push("Three Quarter Moon.".to_string());
        }
    }
    if date.newmoon {
        messages.push("Be careful, New Moon tonight!".to_string());
    }
    if date.fullmoon {
        messages.push("It's a fine day, Full Moon tonight!".to_string());
    }
    if date.summer_solstice {
        messages.push("It's a great day, it's Summer Solstice today!".to_string());
    }
    if date.winter_solstice {
        messages.push("It's a scary day, it's Winter Solstice today!".to_string());
    }
    if date.spring_equinox {
        messages.push("Everything is in balance, it's Spring Equinox today!".to_string());
    }
    if date.fall_equinox {
        messages.push("Everything is in balance, it's Fall Equinox today!".to_string());
    }

    if date.moonday < HALF_MOON_CYCLE {
        messages.push(format!(
            "Next full moon is in {} days.",
            HALF_MOON_CYCLE - date.moonday
        ));
    } else {
        messages.push(format!(
            "Next new moon is in {} days.",
            DAYS_PER_MOON_CYCLE - date.moonday
        ));
    }

    if date.yday < SPRING_EQUINOX_DAY {
        messages.push(format!(
            "Spring Equinox will be in {} days.",
            SPRING_EQUINOX_DAY - date.yday
        ));
    } else if date.yday < SUMMER_SOLSTICE_DAY {
        messages.push(format!(
            "Summer Solstice will be in {} days.",
            SUMMER_SOLSTICE_DAY - date.yday
        ));
    } else if date.yday < FALL_EQUINOX_DAY {
        messages.push(format!(
            "Fall Equinox will be in {} days.",
            FALL_EQUINOX_DAY - date.yday
        ));
    } else {
        messages.push(format!(
            "Winter Solstice will be in {} days.",
            DAYS_PER_YEAR - date.yday
        ));
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

pub(crate) fn apply_help_command(
    command: &str,
    flags: CharacterFlags,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("achelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(anti_cheat_help_lines()));
    }
    if verb.eq_ignore_ascii_case("macrohelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(macro_help_lines()));
    }
    if verb.eq_ignore_ascii_case("penthelp") {
        if !flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(pentagram_help_lines()));
    }
    if !verb.eq_ignore_ascii_case("help") {
        return None;
    }

    let mut messages = vec![
        "=== PLAYER COMMANDS ===".to_string(),
        "== Communication Commands ==".to_string(),
        "/holler <text> - Say something with very long range (costs endurance points)".to_string(),
        "/shout <text> - Say something with extended range (costs endurance points)".to_string(),
        "/say <text> - Make your character say text to nearby players".to_string(),
        "/murmur <text> - Say something with reduced range (whisper alternative)".to_string(),
        "/whisper <text> - Say something with very short range".to_string(),
        "/tell <name> <text> - Send a private message to another player".to_string(),
        "/emote <text> - Express an action (Example: /emote jumps shows Player jumps)".to_string(),
        "/me <text> - Same as /emote (Example:  /me smiles  shows Player smiles)".to_string(),
        "== Emote Shortcuts ==".to_string(),
        "/wave - Wave at others (shortcut for /emote waves happily)".to_string(),
        "/bow - Bow to others (shortcut for /emote bows deeply)".to_string(),
        "/eg - Evil grin (shortcut for /emote grins evilly)".to_string(),
        "/slap <name> - Slap someone with a large trout (humorous emote)".to_string(),
        "/hugme - Show that you need a hug (shortcut for /emote is in need of a hug)".to_string(),
        "== Chat Channel Commands ==".to_string(),
        "/channels - List all available chat channels".to_string(),
        "/join <nr> - Join chat channel number <nr>".to_string(),
        "/leave <nr> - Leave chat channel number <nr>".to_string(),
        "/joinall - Join all channels from 1-13 at once".to_string(),
        "/ah - Various auction house commands".to_string(),
        "== Character & Interaction Commands ==".to_string(),
        "/description <text> - Change your character's description".to_string(),
        "/status - Show your lag control settings and account info".to_string(),
        "/time - Show the current game time and date".to_string(),
        "/weather - Display current weather conditions".to_string(),
        "/swap - Swap places with the player you're facing".to_string(),
        "/allow <name> - Allow another player to search your grave if you die".to_string(),
        "/lastseen <player> - Check when a player last logged into the game".to_string(),
        "/showvalues <player> - Show your stats to another player".to_string(),
        "/who - List all players currently in your area".to_string(),
        "/achievements - View your unlocked achievements".to_string(),
        "/achstats - View your achievement statistics".to_string(),
        "== Command Aliases ==".to_string(),
        "/aliases - Show your active command aliases".to_string(),
        "/alias <short> <long> - Create an alias (Example: \"/alias ty Thank you!\")".to_string(),
        "/alias <short> - Remove an existing alias".to_string(),
        "/clearaliases - Delete ALL your command aliases".to_string(),
        "== PvP & Security Commands ==".to_string(),
        "/playerkiller - Toggle player killing mode on/off".to_string(),
        "/iwilldie <id> - Confirm enabling player killer mode".to_string(),
        "/hate <name> - Add player to your PK list (only works in PK mode)".to_string(),
        "/nohate <name> - Remove player from your PK list".to_string(),
        "/listhate - Show all players on your PK list".to_string(),
        "/clearhate - Clear your entire PK list at once".to_string(),
        "/ignore <name> - Ignore a player in chat and tells".to_string(),
        "/clearignore - Remove ALL players from your ignore list".to_string(),
        "/notells - Toggle receiving private messages on/off".to_string(),
        "/complain <player> [reason] - Report abuse or scamming by a player".to_string(),
        "== Inventory & Gold Commands ==".to_string(),
        "/gold <amount> - Move gold coins to your cursor".to_string(),
        "/sort - Sort items in your inventory by value and type".to_string(),
        "/depotsort - Sort the contents of your storage depot".to_string(),
        "/accountdepotsort - Sort your account-wide storage depot".to_string(),
        "/keyring - View keys stored on your keyring".to_string(),
        "/keyring addall - Add all keys from inventory to keyring".to_string(),
        "/keyring remove <n> - Remove key number <n> from keyring".to_string(),
        "== Clan & Club Commands ==".to_string(),
        "/clan - Show information about the clans".to_string(),
        "/relation <nr> - Show clan <nr>'s diplomatic relations".to_string(),
        "/clanpots - Display information about your clan's potions".to_string(),
        "/clanlog - Check the clan logs (/clanlog -h for more details)".to_string(),
        "/club - Show information about clubs".to_string(),
        "== Character Development Commands ==".to_string(),
        "/set <spell nr> <key> - Change spell key mappings".to_string(),
        "/noexp - Toggle gaining experience on/off".to_string(),
        "/nolevel - Toggle preventing level-ups while continuing to earn exp".to_string(),
        "/hints - Toggle game hints on/off".to_string(),
        "/killbless - Remove all Bless effects from your character".to_string(),
        "== Thief-Specific Commands ==".to_string(),
        "/thief - Toggle thief mode on/off (thief characters only)".to_string(),
        "/steal - Attempt to steal an item from the character you're facing".to_string(),
        "== Game Information Commands ==".to_string(),
        "/orbs - Show available orbs and respawn timers".to_string(),
        "/tunnel <level> - Show progress on a specific tunnel level".to_string(),
        "/tunnels - Show list of all tunnel levels and their status".to_string(),
        "/treasures - Show information on treasures (mine chests, etc.)".to_string(),
        "/demonlords - Show information on demon lords and their status".to_string(),
        "== Lag Control Commands ==".to_string(),
        "/lag - Toggle artificial lag (for testing purposes)".to_string(),
        "/maxlag <seconds> - Set delay for lag control to activate (3-20 seconds)".to_string(),
        "/noball - Toggle using Ball Lightning spell during lag".to_string(),
        "/nobless - Toggle using Bless spell during lag".to_string(),
        "/nofireball - Toggle using Fireball spell during lag".to_string(),
        "/noflash - Toggle using Lightning Flash spell during lag".to_string(),
        "/nofreeze - Toggle using Freeze spell during lag".to_string(),
        "/noheal - Toggle using Heal spell during lag".to_string(),
        "/noshield - Toggle using Magic Shield spell during lag".to_string(),
        "/nowarcry - Toggle using Warcry during lag".to_string(),
        "/nopulse - Toggle using Pulse spell during lag".to_string(),
        "/nolife - Toggle using Healing Potions during lag".to_string(),
        "/nomana - Toggle using Mana Potions during lag".to_string(),
        "/nocombo - Toggle using Combo Potions during lag".to_string(),
        "/norecall - Toggle using Recall Scroll during lag".to_string(),
        "/nomove - Toggle character movement during lag".to_string(),
        "== Automation Commands ==".to_string(),
        "/autobless - Toggle automatic re-blessing when spell expires".to_string(),
        "/autoturn - Toggle automatic turning toward enemies".to_string(),
        "/autopulse - Toggle automatic pulse casting".to_string(),
        "/allowbless - Toggle allowing other players to bless you".to_string(),
        "/killbless - Destroy your own active Bless spell".to_string(),
        "== Miscellaneous Commands ==".to_string(),
        "/logout - Safely log out when standing on a blue square".to_string(),
        "/wimp - Exit from a Live Quest (may have consequences)".to_string(),
        "/help - Display this help text".to_string(),
    ];

    if flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
        messages.extend([
            "=== STAFF COMMANDS ===".to_string(),
            "== Player Management ==".to_string(),
            "/jump <name> <mirror> - Jump to a location or player in specified mirror".to_string(),
            "/look <name> - View a player's character information".to_string(),
            "/values <name> - View a player's stats and values".to_string(),
            "/kick <name> - Disconnect a player from the server".to_string(),
            "/nowho - Hide yourself from /who listings".to_string(),
            "/whostaff - List all staff members online".to_string(),
            "== Disciplinary Actions ==".to_string(),
            "/punish <name> <level> <reason> - Apply punishment to a player".to_string(),
            "/shutup <name> <minutes> - Prevent a player from talking".to_string(),
            "/exterminate <name> - Remove a player from the game".to_string(),
            "/jail <name> - Send a player to jail".to_string(),
            "/unjail <name> - Release a player from jail".to_string(),
            "/klog - Check karma logs".to_string(),
        ]);
    }

    if flags
        .intersects(CharacterFlags::EVENTMASTER | CharacterFlags::LQMASTER | CharacterFlags::GOD)
    {
        messages.push("=== EVENT/QUEST MASTER COMMANDS ===".to_string());
        if flags.contains(CharacterFlags::EVENTMASTER) {
            messages.extend([
                "== Event Master Commands ==".to_string(),
                "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            ]);
        }
        if flags.intersects(CharacterFlags::LQMASTER | CharacterFlags::GOD) {
            messages.extend([
                "== Quest Master Commands ==".to_string(),
                "/immortal - Toggle immortality status".to_string(),
                "/infrared - Toggle infrared vision".to_string(),
                "/invisible - Toggle invisibility".to_string(),
            ]);
            if area_id == 20 || area_id == 35 {
                messages.push(
                    "Note: Additional LQ commands are available in the Live Quest area".to_string(),
                );
            }
        }
    }

    if flags.contains(CharacterFlags::GOD) {
        messages.extend([
            "=== GOD COMMANDS ===".to_string(),
            "== Movement & Teleportation ==".to_string(),
            "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            "/gotolist - List all available goto locations".to_string(),
            "/gotosearch <term> - Search for goto locations".to_string(),
            "/office - Teleport to staff office in Aston".to_string(),
            "/summon <name> - Bring a player to your location".to_string(),
            "/summonall - Bring all online players to your location".to_string(),
            "== Item Management ==".to_string(),
            "/create <name> - Create an item by template name".to_string(),
            "/create_orb [type] [value] - Create an orb with specific properties".to_string(),
            "/itemmod <pos> <skill> <val> - Modify item in cursor (position, skill, value)"
                .to_string(),
            "/itemname <name> - Change name of item in cursor".to_string(),
            "/itemdesc <text> - Change description of item in cursor".to_string(),
            "/listitem <id> - Show detailed information about an item".to_string(),
            "== Player Modification ==".to_string(),
            "/ggold <amount> - Give yourself gold coins".to_string(),
            "/exp [name] [amount] - Give experience to a player".to_string(),
            "/milexp [name] [amount] - Give military experience to a player".to_string(),
            "/setskill <name> <skill> <value> - Set a player's skill level".to_string(),
            "/setlevel <level> - Set your character level".to_string(),
            "/heal - Fully restore your health".to_string(),
            "/setseyan <name> - Make a player a Seyan'Du".to_string(),
            "/rmdeath <name> - Remove one death from player's record".to_string(),
            "/setkarma <name> <value> - Set a player's karma".to_string(),
            "/toggleflag <name> <flag> - Toggles a flag for a character - use with caution"
                .to_string(),
            "/saves <amount> - Set number of saves".to_string(),
            "== Quest & Progress Management ==".to_string(),
            "/resetgift <name> <area> - Reset a player's gift status for an area".to_string(),
            "/fixit <name> - Fix a player's questlog".to_string(),
            "/questfix <name> - Fix quests for a player".to_string(),
            "/reset <name> - Reset a player's skills".to_string(),
            "/noarch <name> - Remove arch status from a player".to_string(),
            "/noprof <name> - Remove professions from a player".to_string(),
            "/questlog <name> - View a player's quest log".to_string(),
            "/labsolved <name> [lab] - Show or toggle lab completion status".to_string(),
            "== Achievements ==".to_string(),
            "/achgive <name> <id> - Award achievement to player".to_string(),
            "/achfix [name] - Recheck and award earned achievements".to_string(),
            "/achclear [name] - Clear all achievements (dev only)".to_string(),
            "/achsync [name] - Force sync achievements to client".to_string(),
            "== Account Management ==".to_string(),
            "/rename <oldname> <newname> - Rename a player character".to_string(),
            "/lockname <name> - Lock a character name".to_string(),
            "/unlockname <name> - Unlock a character name".to_string(),
            "/unpunish <name> <id> - Remove a punishment".to_string(),
            "== Character Information ==".to_string(),
            "/showppd <name> <ppd> - Show player persistent data".to_string(),
            "#ls <name> <dir> - Ask a player's client to list a local directory".to_string(),
            "#cat <name> <file> - Ask a player's client to send a local file's contents"
                .to_string(),
            "/showflags <name> - Show which flags are enabled on a character".to_string(),
            "/listchars - List all active characters".to_string(),
            "== God Status Management ==".to_string(),
            "/immortal - Toggle immortality status".to_string(),
            "/invisible - Toggle invisibility".to_string(),
            "/infrared - Toggle infrared vision".to_string(),
            "/xray - Toggle x-ray vision mode".to_string(),
            "/sprite <num> - Change your sprite".to_string(),
            "/color - Show your color values".to_string(),
            "/col1 <r> <g> <b> - Set your primary colors".to_string(),
            "/col2 <r> <g> <b> - Set your secondary colors".to_string(),
            "/col3 <r> <g> <b> - Set your tertiary colors".to_string(),
            "/dlight <value> - Override dynamic lighting".to_string(),
            "/showattack - Toggle attack display".to_string(),
            "/spy - Toggle spy mode (see all tells, clan, alliance, club, area, mirror chat)"
                .to_string(),
            "== Server Management ==".to_string(),
            "/shutdown <minutes> <downtime> - Schedule server shutdown".to_string(),
            "/respawn - Force respawn check".to_string(),
            "/setxmas <value> - Set Christmas special flag".to_string(),
            "/global - Display current global game settings".to_string(),
            "/checksanity - Run consistency checks on game data".to_string(),
            "/saveall - Force save of all player data".to_string(),
            "== Diagnostics & Monitoring ==".to_string(),
            "/memstats - Show memory usage statistics".to_string(),
            "/profinfo - Show profiling information".to_string(),
            "/poolstats - Show database connection pool statistics".to_string(),
            "/querystats - Show database query statistics".to_string(),
            "/prof - Show memory profiling information".to_string(),
            "== Game Settings Management ==".to_string(),
            "/setexpmod <value> - Set global experience modifier".to_string(),
            "/sethardcoreexpbonus <value> - Set hardcore experience bonus".to_string(),
            "/sethardcoremilexpbonus <value> - Set hardcore military exp bonus".to_string(),
            "/sethardcorekillexpbonus <value> - Set hardcore kill exp bonus".to_string(),
            "/setdecaytime <ticks> - Set item decay time".to_string(),
            "/setplayerbodytime <ticks> - Set player body decay time".to_string(),
            "/setnpcbodytime <ticks> - Set NPC body decay time".to_string(),
            "/setnpcbodytimearea32 <ticks> - Set area 32 NPC body decay time".to_string(),
            "/setrespawntime <ticks> - Set NPC respawn time".to_string(),
            "/setlagouttime <ticks> - Set lagout time".to_string(),
            "/setregentime <ticks> - Set regeneration time".to_string(),
            "/setsewerrespawntime <seconds> - Set sewer item respawn time".to_string(),
            "== Communication Settings ==".to_string(),
            "/sethollerdist <tiles> - Set holler distance".to_string(),
            "/setshoutdist <tiles> - Set shout distance".to_string(),
            "/setsaydist <tiles> - Set say distance".to_string(),
            "/setemotedist <tiles> - Set emote distance".to_string(),
            "/setquietsaydist <tiles> - Set quiet say distance".to_string(),
            "/setwhisperdist <tiles> - Set whisper distance".to_string(),
            "/sethollercost <points> - Set holler endurance cost".to_string(),
            "/setshoutcost <points> - Set shout endurance cost".to_string(),
            "== Special Item Settings ==".to_string(),
            "/setsplots <value> - Set special item probability 'lots'".to_string(),
            "/setspmany <value> - Set special item probability 'many'".to_string(),
            "/setspsome <value> - Set special item probability 'some'".to_string(),
            "/setspfew <value> - Set special item probability 'few'".to_string(),
            "/setsprare <value> - Set special item probability 'rare'".to_string(),
            "/setspultra <value> - Set special item probability 'ultra'".to_string(),
            "== Orb & Tunnel Management ==".to_string(),
            "/setorbrespawndays <days> - Set orb respawn time".to_string(),
            "/settunnelexpdivider <value> - Set tunnel exp base value divider".to_string(),
            "/settunnelmillexp <value> - Set tunnel mill exp base value".to_string(),
            "/changetunnel <name> <level> - Change player's tunnel level".to_string(),
            "/settunnel <name> <level> <amount> - Set completion amount for tunnel".to_string(),
            "/cleartunnel <name> <level> - Clear tunnel completion status".to_string(),
            "/solvetunnel <type> - Simulate solving the current tunnel".to_string(),
            "== Shrine & Dungeon Management ==".to_string(),
            "/setrd <name> <number> - Set continuity shrine number".to_string(),
            "/clearrd <name> <number> - Clear used shrine bits".to_string(),
            "/solverd <name> <number> - Mark non-continuity shrines as used".to_string(),
            "== Clan & Club Management ==".to_string(),
            "/killclan <nr> - Destroy a clan".to_string(),
            "/killclub <nr> - Destroy a club".to_string(),
            "/joinclan <nr> - Join a specific clan".to_string(),
            "/joinclub <nr> - Join a specific club".to_string(),
            "/setmaxjewelcount <value> - Set maximum clan jewel count".to_string(),
            "/clearclanlog <clan> - Clear the clan log for a specific clan".to_string(),
            "/setclanjewels <clan> <count> [log] - Set clan jewel count".to_string(),
            "/renclan <nr> <name> - Rename clan with specified number".to_string(),
            "/renclub <nr> <name> - Rename club with specified number".to_string(),
            "== Military Administration ==".to_string(),
            "/milinfo [name] - View a player's military data and mission status".to_string(),
            "/milpref <name> <type> <difficulty> - Set a player's mission preferences".to_string(),
            "/milreset [name] - Reset a player's mission cooldowns and advisor timers".to_string(),
            "/milpoints <name> <points> - Grant military points to a player".to_string(),
            "/milrec <name> <points> - Grant recommendation points to a player".to_string(),
            "/milstats - View statistics about the military system".to_string(),
            "/milsolve [name] [announce] - Complete a player's current military mission"
                .to_string(),
            "== Weather System Management ==".to_string(),
            "/setweather <type> <intensity> - Set global weather".to_string(),
            "/clearweather - Clear weather globally".to_string(),
            "/setareaweather <area> <type> - Set weather for specific area".to_string(),
            "== Player Status Management ==".to_string(),
            "/god <name> - Toggle god status for a player".to_string(),
            "/staff <name> - Toggle staff status for a player".to_string(),
            "/staffcode <name> <code> - Set staff code for a player".to_string(),
            "/qmaster <name> - Toggle quest master status".to_string(),
            "/emaster <name> - Toggle event master status".to_string(),
            "/devel <name> - Toggle developer status".to_string(),
            "/setsir <name> - Toggle sir/lady status".to_string(),
            "/hardcore <name> - Toggle hardcore mode for a player".to_string(),
            "== Miscellaneous God Commands ==".to_string(),
            "/laugh - Play laugh sound effect".to_string(),
            "/ls <name> <file> - List files for a player".to_string(),
            "/cat <name> <file> - View file content for a player".to_string(),
            "/lollipop <name> - Send lollipop to a player".to_string(),
            "/clearmerchantstores <id> - Reset a merchant's inventory".to_string(),
        ]);
    }

    messages.push(
        "Type a command without parameters to get more information in some cases.".to_string(),
    );

    Some(legacy_help_result(messages))
}

pub(crate) fn anti_cheat_help_lines() -> Vec<String> {
    [
        "--- Anti-Cheat Commands ---",
        "#achelp - Show this help",
        "#acstats - Global AC statistics",
        "#aclist - List online players with AC status",
        "#acsuspicious - List suspicious/flagged players",
        "--- Player Commands ---",
        "#acstatus <name> - Show player's AC status",
        "#achistory <name> - Show player's violation history",
        "#acsessions <name> - Show player's recent sessions",
        "#acviolations <name> - Show player's violations",
        "#acflag <name> - Flag player for review",
        "#acunflag <name> - Remove flagged status",
        "#actrust <name> - Mark player as trusted",
        "#acuntrust <name> - Remove trusted status",
        "#acreset <name> - Reset player's AC data (God)",
        "#acwarn <name> [reason] - Issue AC warning",
        "#acwatch <name> - Toggle detailed logging",
        "--- Multi-Account Detection ---",
        "#acsharedip <name> - Show accounts sharing IP",
        "#acsharedhw <name> - Show accounts sharing hardware",
        "--- Database Queries ---",
        "#achighrisk - Show high-risk players",
        "#aclookup <id> - Lookup by subscriber ID",
        "--- Signature Management ---",
        "#acsiglist - List known bad signatures",
        "#acsigadd <type> <value> <name> - Add signature (God)",
        "#acsigdel <id> - Delete signature (God)",
        "--- Maintenance ---",
        "#accleanup <days> - Cleanup old records (God)",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn macro_help_lines() -> Vec<String> {
    [
        "=== Macro Daemon Admin Commands ===",
        "/macrostats <player> - Show player's macro stats",
        "/macrohistory <player> - Show challenge history",
        "/macrolist - List all players with macro status",
        "/summonmacro <player> - Force summon (GOD only)",
        "/macroimmune <player> <mins> - Grant immunity (GOD only)",
        "/macrosuspicion <player> <amt> - Adjust suspicion (GOD)",
        "/macrokarma <player> <val> - Set karma 0-100 (GOD)",
        "/macrofailures <player> <n> - Set failure count (GOD)",
        "/macroreset <player> - Reset all macro stats (GOD)",
        "/macrohelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn pentagram_help_lines() -> Vec<String> {
    [
        "=== Pentagram Debug Commands (GOD) ===",
        "/pentinfo <player> - Show pentagram data",
        "/setpentcount <player> <n> - Set pent_cnt (run count)",
        "/setpentstatus <player> <0|1> - Set status",
        "/setpentbonus <player> <n> - Set bonus exp",
        "/resetpent <player> - Reset all pent data",
        "/penthelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
