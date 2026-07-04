use super::*;

pub(crate) const SWEAR_LASTTALK_OFFSET: usize = 0;

pub(crate) const SWEAR_BAD_OFFSET: usize = SWEAR_LASTTALK_OFFSET + 10 * 4;

pub(crate) const SWEAR_SENTENCES_OFFSET: usize = SWEAR_BAD_OFFSET + 4;

pub(crate) const SWEAR_LAST_TIME_OFFSET: usize =
    SWEAR_SENTENCES_OFFSET + SWEAR_SENTENCE_COUNT * SWEAR_SENTENCE_LEN;

pub(crate) const SWEAR_LAST_CNT_OFFSET: usize = SWEAR_LAST_TIME_OFFSET + 10 * 4;

pub(crate) const SWEAR_LAST_POS_OFFSET: usize = SWEAR_LAST_CNT_OFFSET + 10 * 4;

pub(crate) const SWEAR_BANNED_TILL_OFFSET: usize = LEGACY_SWEAR_PPD_SIZE - 4;

pub(crate) fn read_swear_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub(crate) fn write_swear_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn ensure_swear_ppd(player: &mut PlayerRuntime) -> &mut Vec<u8> {
    if player.swear_ppd.len() < LEGACY_SWEAR_PPD_SIZE {
        player.swear_ppd.resize(LEGACY_SWEAR_PPD_SIZE, 0);
    }
    &mut player.swear_ppd
}

pub(crate) fn legacy_all_upper(text: &str) -> bool {
    let mut alpha_count = 0;
    for byte in text.bytes() {
        if byte.is_ascii_lowercase() {
            return false;
        }
        if byte.is_ascii_alphabetic() {
            alpha_count += 1;
        }
    }
    alpha_count > 3
}

pub(crate) fn legacy_swear_block(
    player: &mut PlayerRuntime,
    realtime_seconds: u64,
    messages: &[&str],
) -> Vec<Vec<u8>> {
    let realtime = realtime_seconds.min(i32::MAX as u64) as i32;
    let ppd = ensure_swear_ppd(player);
    write_swear_i32(ppd, SWEAR_BAD_OFFSET, realtime);
    messages
        .iter()
        .map(|message| legacy_light_red_text_bytes(message))
        .collect()
}

pub(crate) fn legacy_swearing_feedback(
    player: &mut PlayerRuntime,
    is_player: bool,
    is_god: bool,
    text: &str,
    realtime_seconds: u64,
) -> Option<Vec<Vec<u8>>> {
    if !is_player {
        return None;
    }

    let realtime = realtime_seconds.min(i32::MAX as u64) as i32;
    let ppd = ensure_swear_ppd(player);
    let banned_till = read_swear_i32(ppd, SWEAR_BANNED_TILL_OFFSET);
    if banned_till > realtime {
        let minutes = f64::from(banned_till - realtime) / 60.0;
        return Some(vec![legacy_light_red_text_bytes(&format!(
            "Chat is blocked for {minutes:.2} minutes."
        ))]);
    }

    if is_god {
        return None;
    }

    let bad = read_swear_i32(ppd, SWEAR_BAD_OFFSET);
    if realtime - bad < 30 {
        return Some(vec![legacy_light_red_text_bytes("Chat is blocked.")]);
    }
    if realtime - read_swear_i32(ppd, SWEAR_LASTTALK_OFFSET + 4) < 1 {
        return Some(legacy_swear_block(
            player,
            realtime_seconds,
            &["Chat has been blocked for 30 seconds for excessive usage (1)."],
        ));
    }
    if realtime - read_swear_i32(ppd, SWEAR_LASTTALK_OFFSET + 4 * 4) < 10 {
        return Some(legacy_swear_block(
            player,
            realtime_seconds,
            &["Chat has been blocked for 30 seconds for excessive usage (2)."],
        ));
    }
    if realtime - read_swear_i32(ppd, SWEAR_LASTTALK_OFFSET + 9 * 4) < 30 {
        return Some(legacy_swear_block(
            player,
            realtime_seconds,
            &["Chat has been blocked for 30 seconds for excessive usage (3)."],
        ));
    }

    let lower = text.to_ascii_lowercase();
    if ["fuck", "cunt", "faggot", "korwa", "nigga"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Some(legacy_swear_block(
            player,
            realtime_seconds,
            &[
                "Swearing is illegal in this game. While only a few words are blocked by the system, you will get punished and eventually banned if you swear using non-blocked words.",
                "Chat has been blocked for 30 seconds.",
            ],
        ));
    }

    if text.len() > 3 && legacy_all_upper(text) {
        return Some(legacy_swear_block(
            player,
            realtime_seconds,
            &[
                "Using capitalized letters only is impolite. Trying to get around the block by using mostly caps will get you punished and eventually banned.",
                "Chat has been blocked for 30 seconds.",
            ],
        ));
    }

    if text.len() > 20 {
        let mut found = false;
        let compare_len = text.len().min(78);
        for index in 0..SWEAR_SENTENCE_COUNT {
            let sentence_offset = SWEAR_SENTENCES_OFFSET + index * SWEAR_SENTENCE_LEN;
            let stored = &ppd[sentence_offset..sentence_offset + compare_len];
            if stored == &text.as_bytes()[..compare_len]
                && realtime - read_swear_i32(ppd, SWEAR_LAST_TIME_OFFSET + index * 4) < 30
            {
                if read_swear_i32(ppd, SWEAR_LAST_CNT_OFFSET + index * 4) > 2
                    || realtime - read_swear_i32(ppd, SWEAR_LAST_TIME_OFFSET + index * 4) < 4
                {
                    return Some(legacy_swear_block(
                        player,
                        realtime_seconds,
                        &[
                            "Repeating the same sentence is impolite. Repeating variants of the same sentence will get you punished and eventually banned.",
                            "Chat has been blocked for 30 seconds.",
                        ],
                    ));
                }
                let count_offset = SWEAR_LAST_CNT_OFFSET + index * 4;
                let count = read_swear_i32(ppd, count_offset).saturating_add(1);
                write_swear_i32(ppd, count_offset, count);
                let last_pos = read_swear_i32(ppd, SWEAR_LAST_POS_OFFSET);
                if (0..10).contains(&last_pos) {
                    write_swear_i32(
                        ppd,
                        SWEAR_LAST_TIME_OFFSET + last_pos as usize * 4,
                        realtime,
                    );
                }
                found = true;
                break;
            }
        }
        if !found {
            let mut last_pos = read_swear_i32(ppd, SWEAR_LAST_POS_OFFSET);
            if !(0..=9).contains(&last_pos) {
                last_pos = 0;
            }
            let sentence_offset = SWEAR_SENTENCES_OFFSET + last_pos as usize * SWEAR_SENTENCE_LEN;
            ppd[sentence_offset..sentence_offset + SWEAR_SENTENCE_LEN].fill(0);
            let copy_len = text.len().min(78);
            ppd[sentence_offset..sentence_offset + copy_len]
                .copy_from_slice(&text.as_bytes()[..copy_len]);
            write_swear_i32(
                ppd,
                SWEAR_LAST_TIME_OFFSET + last_pos as usize * 4,
                realtime,
            );
            write_swear_i32(ppd, SWEAR_LAST_CNT_OFFSET + last_pos as usize * 4, 1);
            write_swear_i32(ppd, SWEAR_LAST_POS_OFFSET, last_pos + 1);
        }
    }

    for index in (1..10).rev() {
        let previous = read_swear_i32(ppd, SWEAR_LASTTALK_OFFSET + (index - 1) * 4);
        write_swear_i32(ppd, SWEAR_LASTTALK_OFFSET + index * 4, previous);
    }
    write_swear_i32(ppd, SWEAR_LASTTALK_OFFSET, realtime);
    None
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TellCommandResult {
    pub(crate) sender_messages: Vec<String>,
    pub(crate) delivered_messages: Vec<(CharacterId, String)>,
    pub(crate) delivered_message_bytes: Vec<(CharacterId, Vec<u8>)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ChatCommandResult {
    pub(crate) sender_messages: Vec<String>,
    pub(crate) delivered_message_bytes: Vec<(CharacterId, Vec<u8>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalSpeechKind {
    Emote,
    Holler,
    Shout,
    Say,
    Murmur,
    Whisper,
}

impl LocalSpeechKind {
    fn from_verb(verb: &str) -> Option<Self> {
        match verb.to_ascii_lowercase().as_str() {
            "holler" => Some(Self::Holler),
            "shout" => Some(Self::Shout),
            "say" => Some(Self::Say),
            "murmur" => Some(Self::Murmur),
            "whisper" => Some(Self::Whisper),
            _ => None,
        }
    }

    fn max_distance(self, runtime: &ServerRuntime) -> i32 {
        match self {
            Self::Emote => runtime.emote_dist,
            Self::Holler => runtime.holler_dist,
            Self::Shout => runtime.shout_dist,
            Self::Say => runtime.say_dist,
            Self::Murmur => runtime.quietsay_dist,
            Self::Whisper => runtime.whisper_dist,
        }
    }

    fn endurance_cost(self, runtime: &ServerRuntime) -> i32 {
        match self {
            Self::Holler => runtime.holler_cost,
            Self::Shout => runtime.shout_cost,
            Self::Emote | Self::Say | Self::Murmur | Self::Whisper => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChatChannelInfo {
    number: u8,
    name: &'static str,
    description: &'static str,
}

pub(crate) const LEGACY_MAX_CLAN: i64 = 32;

pub(crate) const LEGACY_MAX_CLUB: i64 = 16_384;

pub(crate) const LEGACY_CLUB_OFFSET: i64 = 1_024;

pub(crate) const LEGACY_CHAT_CHANNELS: &[ChatChannelInfo] = &[
    ChatChannelInfo {
        number: 0,
        name: "Announce",
        description: "Announcements from management - NOLEAVE",
    },
    ChatChannelInfo {
        number: 1,
        name: "Info",
        description: "Requesting staff help, technical and gameplay questions",
    },
    ChatChannelInfo {
        number: 2,
        name: "Gossip",
        description: "Talk about Life, the Universe and Everything",
    },
    ChatChannelInfo {
        number: 3,
        name: "Auction",
        description: "Buy and sell stuff",
    },
    ChatChannelInfo {
        number: 4,
        name: "Astonia",
        description: "Other Astonia versions (2.0, 3.5)",
    },
    ChatChannelInfo {
        number: 5,
        name: "Clan",
        description: "Public channel for clan related matters",
    },
    ChatChannelInfo {
        number: 6,
        name: "Grats",
        description: "Grats on leveling!",
    },
    ChatChannelInfo {
        number: 7,
        name: "Clan2",
        description: "Channel only visible to members of your clan",
    },
    ChatChannelInfo {
        number: 8,
        name: "Area",
        description: "Channel only visible to those in your area",
    },
    ChatChannelInfo {
        number: 9,
        name: "Mirror",
        description: "Only visible to those in your area and mirror",
    },
    ChatChannelInfo {
        number: 10,
        name: "Games",
        description: "Discussions of computer games",
    },
    ChatChannelInfo {
        number: 11,
        name: "Kill",
        description: "Playerkiller related topics",
    },
    ChatChannelInfo {
        number: 12,
        name: "ClanA",
        description: "Channel only visible to clan members and allies",
    },
    ChatChannelInfo {
        number: 13,
        name: "Club",
        description: "Channel only visible to your club members",
    },
    ChatChannelInfo {
        number: 14,
        name: "Development",
        description: "Channel only visible to developers",
    },
    ChatChannelInfo {
        number: 31,
        name: "Staff",
        description: "Staff member's private channel",
    },
    ChatChannelInfo {
        number: 32,
        name: "God",
        description: "Ye God's private channel",
    },
];

pub(crate) fn apply_shutup_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("shutup") {
        return None;
    }

    let caller = world.characters.get(&character_id)?;
    if !caller
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return None;
    }

    let rest = rest.trim_start();
    let (name, minute_text) = take_legacy_alpha_name(rest);
    let minutes = if minute_text.trim_start().is_empty() {
        10
    } else {
        legacy_atoi_prefix(minute_text.trim_start())
    };

    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(KeyringCommandResult {
            messages: vec![format!("Sorry, no player by the name {name}.")],
            ..Default::default()
        });
    };
    if !world
        .characters
        .get(&target_id)
        .is_some_and(|target| target.flags.contains(CharacterFlags::PLAYER))
    {
        return Some(KeyringCommandResult {
            messages: vec![format!("Sorry, no player by the name {name}.")],
            ..Default::default()
        });
    }

    if !(0..=60).contains(&minutes) {
        return Some(KeyringCommandResult {
            messages: vec![
                "Sorry, can only shutup for 0 to 60 minutes (use 0 to disable).".to_string(),
            ],
            ..Default::default()
        });
    }

    if let Some(target) = world.characters.get_mut(&target_id) {
        if minutes == 0 {
            target.flags.remove(CharacterFlags::SHUTUP);
        } else {
            target.flags.insert(CharacterFlags::SHUTUP);
        }
    }
    if let Some(target_player) = runtime.player_for_character_mut(target_id) {
        target_player.shutup_until_seconds = if minutes == 0 {
            0
        } else {
            realtime_seconds.saturating_add(minutes as u64 * 60)
        };
    }

    let message = if minutes == 0 {
        "Your ability to talk has been enabled."
    } else {
        "Your ability to talk has been disabled."
    };
    Some(KeyringCommandResult {
        target_message_bytes: vec![(target_id, legacy_light_red_text_bytes(message))],
        ..Default::default()
    })
}

pub(crate) fn drain_expired_shutup_feedback(
    world: &mut World,
    runtime: &mut ServerRuntime,
    realtime_seconds: u64,
) -> Vec<(CharacterId, Vec<u8>)> {
    let mut feedback = Vec::new();
    let expired: Vec<CharacterId> = runtime
        .players
        .values_mut()
        .filter_map(|player| {
            let character_id = player.character_id?;
            (player.shutup_until_seconds != 0 && player.shutup_until_seconds <= realtime_seconds)
                .then(|| {
                    player.shutup_until_seconds = 0;
                    character_id
                })
        })
        .collect();

    for character_id in expired {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.flags.remove(CharacterFlags::SHUTUP);
        }
        feedback.push((
            character_id,
            legacy_light_red_text_bytes("Your ability to talk has been enabled."),
        ));
    }

    feedback
}

pub(crate) fn apply_notells_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"notells".starts_with(&lower) {
        return None;
    }

    let character = world.characters.get_mut(&character_id)?;
    character.flags.toggle(CharacterFlags::NOTELL);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Turned no-tell mode {}.",
            if character.flags.contains(CharacterFlags::NOTELL) {
                "on"
            } else {
                "off"
            }
        )],
        ..Default::default()
    })
}

pub(crate) fn chat_command_verb(command: &str) -> (&str, &str) {
    if !command.starts_with('/') && !command.starts_with('#') {
        return ("say", command);
    }
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    (verb.trim_start_matches('/').trim_start_matches('#'), rest)
}

pub(crate) fn legacy_cmd_prefix(verb: &str, full: &str, min_len: usize) -> bool {
    let verb = verb.to_ascii_lowercase();
    verb.len() >= min_len && full.starts_with(&verb)
}

pub(crate) fn legacy_emote_text(verb: &str, raw_text: &str, underwater: bool) -> Option<String> {
    if legacy_cmd_prefix(verb, "emote", 2) || legacy_cmd_prefix(verb, "me", 2) {
        return Some(if underwater {
            "feels wet".to_string()
        } else {
            raw_text.trim_start().to_string()
        });
    }
    if legacy_cmd_prefix(verb, "slap", 4) {
        return Some(format!(
            "slaps {} around a bit with a large trout",
            raw_text.trim_start()
        ));
    }
    if legacy_cmd_prefix(verb, "wave", 2) {
        return Some("waves happily".to_string());
    }
    if legacy_cmd_prefix(verb, "hugme", 5) {
        return Some("is in need of a hug".to_string());
    }
    if legacy_cmd_prefix(verb, "bow", 2) {
        return Some("bows deeply".to_string());
    }
    if legacy_cmd_prefix(verb, "eg", 2) {
        return Some("grins evilly".to_string());
    }
    None
}

pub(crate) fn apply_demon_ritual_speech(
    world: &mut World,
    sender_id: CharacterId,
    text: &str,
) -> Vec<String> {
    let Some(sender) = world.characters.get_mut(&sender_id) else {
        return Vec::new();
    };
    let spoken = text.trim();
    for ritual in 0..5 {
        if ugaris_core::item_driver::demon_ritual_words(sender.id.0, ritual)
            .eq_ignore_ascii_case(spoken)
        {
            let cap = i16::try_from((ritual + 1) * 5).unwrap_or(i16::MAX);
            let effective = sender.values[1][CharacterValue::Demon as usize];
            sender.values[0][CharacterValue::Demon as usize] = cap.min(effective);
            sender.flags.insert(CharacterFlags::UPDATE);
            let mut messages = vec!["You intone the protective ritual.".to_string()];
            if cap < effective {
                messages.push(
                    "You sense that this ritual cannot utilize your full knowledge.".to_string(),
                );
            }
            return messages;
        }
    }
    Vec::new()
}

pub(crate) fn legacy_chat_channel(number: u8) -> Option<ChatChannelInfo> {
    LEGACY_CHAT_CHANNELS
        .iter()
        .copied()
        .find(|channel| channel.number == number)
}

pub(crate) fn legacy_chat_channel_color(channel: u8) -> u8 {
    match channel {
        0 => 3,
        1 => 12,
        2 => 2,
        3 => 9,
        4 => 14,
        5 => 15,
        6 => 10,
        7 => 16,
        8 => 13,
        9 => 11,
        10 | 11 => 14,
        12 | 13 => 16,
        14 => 11,
        31 => 7,
        32 => 8,
        _ => 2,
    }
}

pub(crate) fn legacy_chat_command_channel(command: &str) -> Option<(u8, &str)> {
    let (verb, rest) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    for channel in LEGACY_CHAT_CHANNELS {
        let alias = format!("c{}", channel.number);
        if alias.starts_with(&lower) || channel.name.to_ascii_lowercase().starts_with(&lower) {
            return Some((channel.number, rest));
        }
    }
    None
}

pub(crate) fn legacy_chat_line(
    sender: &Character,
    staff_code: &str,
    mirror: u16,
    channel: ChatChannelInfo,
    text: &str,
) -> Vec<u8> {
    let color = legacy_chat_channel_color(channel.number);
    if channel.number == 0 {
        let mut out = runtime_color(color);
        out.extend_from_slice(text.as_bytes());
        return out;
    }

    let mut sender_name = if sender.flags.contains(CharacterFlags::STAFF) {
        sender.name.to_ascii_uppercase()
    } else {
        sender.name.clone()
    };
    sender_name.truncate(75);

    let player_color = if sender.flags.contains(CharacterFlags::GOD) {
        COL_LIGHT_RED
    } else if sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::EVENTMASTER)
    {
        COL_LIGHT_GREEN
    } else {
        COL_RESET
    };

    let mut out = runtime_color(color);
    out.extend_from_slice(channel.name.as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(player_color);
    out.extend_from_slice(sender_name.as_bytes());
    out.extend_from_slice(&runtime_color(color));
    if sender.flags.contains(CharacterFlags::STAFF) && !sender.flags.contains(CharacterFlags::GOD) {
        out.extend_from_slice(b"[");
        out.extend_from_slice(staff_code.as_bytes());
        out.extend_from_slice(b"]");
    }
    out.extend_from_slice(b" ");
    if channel.number == 4 {
        out.extend_from_slice(format!("(OW) says: \"{text}\"").as_bytes());
    } else {
        out.extend_from_slice(format!("({mirror}) says: \"{text}\"").as_bytes());
    }
    out
}

pub(crate) fn legacy_spy_line(kind: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(COL_DARK_GRAY.len() + kind.len() + payload.len() + 8);
    out.extend_from_slice(COL_DARK_GRAY);
    out.extend_from_slice(b"[SPY/");
    out.extend_from_slice(kind.as_bytes());
    out.extend_from_slice(b"] ");
    out.extend_from_slice(payload);
    out.extend_from_slice(COL_RESET);
    out
}

pub(crate) fn local_speech_payload(
    kind: LocalSpeechKind,
    name: &str,
    text: &str,
) -> Option<Vec<u8>> {
    match kind {
        LocalSpeechKind::Emote => emote_message(name, text),
        LocalSpeechKind::Holler => holler_message(name, text),
        LocalSpeechKind::Shout => shout_message(name, text),
        LocalSpeechKind::Say => Some(say_message(name, text)),
        LocalSpeechKind::Murmur => (!text.contains('"'))
            .then(|| sanitize_log_bytes(format!("{name} murmurs: \"{text}\"").as_bytes())),
        LocalSpeechKind::Whisper => whisper_message(name, text),
    }
}

pub(crate) fn apply_local_speech_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
    realtime_seconds: u64,
) -> Option<ChatCommandResult> {
    let (verb, raw_text) = chat_command_verb(command);
    let is_plain_speech = !command.starts_with('/') && !command.starts_with('#');

    let sender = world.characters.get(&sender_id)?;
    if sender.flags.contains(CharacterFlags::SHUTUP) {
        return Some(ChatCommandResult {
            sender_messages: vec!["Sorry, you cannot say anything right now.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let underwater = world
        .map
        .tile(usize::from(sender.x), usize::from(sender.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::UNDERWATER));

    let emote_text = legacy_emote_text(verb, raw_text, underwater);
    let kind = if emote_text.is_some() {
        LocalSpeechKind::Emote
    } else {
        LocalSpeechKind::from_verb(verb)?
    };
    let text = if let Some(text) = emote_text.as_deref() {
        text
    } else if underwater {
        "Blub."
    } else {
        raw_text.trim_start()
    };
    let actual_kind = if underwater && kind != LocalSpeechKind::Emote {
        LocalSpeechKind::Say
    } else {
        kind
    };

    if let Some(player) = runtime.player_for_character_mut(sender_id) {
        if let Some(messages) = legacy_swearing_feedback(
            player,
            sender.flags.contains(CharacterFlags::PLAYER),
            sender.flags.contains(CharacterFlags::GOD),
            text,
            realtime_seconds,
        ) {
            return Some(ChatCommandResult {
                sender_messages: Vec::new(),
                delivered_message_bytes: messages
                    .into_iter()
                    .map(|message| (sender_id, message))
                    .collect(),
            });
        }
    }

    let cost = actual_kind.endurance_cost(runtime);
    if cost > 0 && sender.endurance < cost {
        let message = match actual_kind {
            LocalSpeechKind::Holler => "You're too exhausted to holler.",
            LocalSpeechKind::Shout => "You're too exhausted to shout.",
            _ => unreachable!(),
        };
        return Some(ChatCommandResult {
            sender_messages: vec![message.to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let Some(payload) = local_speech_payload(actual_kind, &sender.name, text) else {
        return Some(ChatCommandResult::default());
    };
    let max_distance = actual_kind.max_distance(runtime);
    let sender_x = i32::from(sender.x);
    let sender_y = i32::from(sender.y);

    if cost > 0 {
        if let Some(sender) = world.characters.get_mut(&sender_id) {
            sender.endurance = sender.endurance.saturating_sub(cost);
            sender.regen_ticker = u32::try_from(current_tick).unwrap_or(u32::MAX);
        }
    }

    let sender_messages = if is_plain_speech && !underwater {
        apply_demon_ritual_speech(world, sender_id, text)
    } else {
        Vec::new()
    };

    // C `say` -> notify_area(..., NT_TEXT, ...): NPC drivers such as
    // merchants react to nearby player speech.
    if is_plain_speech && !underwater {
        let npc_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| {
                !character.flags.contains(CharacterFlags::PLAYER)
                    && character.driver != 0
                    && character.id != sender_id
                    && (i32::from(character.x) - sender_x).abs() <= 16
                    && (i32::from(character.y) - sender_y).abs() <= 16
            })
            .map(|character| character.id)
            .collect();
        for npc_id in npc_ids {
            if let Some(npc) = world.characters.get_mut(&npc_id) {
                npc.push_driver_text_message(sender_id, text);
            }
        }
    }

    let mut delivered_message_bytes = Vec::new();
    for player in runtime.players.values() {
        let Some(target_id) = player.character_id else {
            continue;
        };
        let Some(target) = world.characters.get(&target_id) else {
            continue;
        };
        if !target
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::USED)
        {
            continue;
        }
        if (i32::from(target.x) - sender_x).abs() > max_distance
            || (i32::from(target.y) - sender_y).abs() > max_distance
        {
            continue;
        }
        delivered_message_bytes.push((target_id, payload.clone()));
    }

    Some(ChatCommandResult {
        sender_messages,
        delivered_message_bytes,
    })
}

pub(crate) fn apply_chat_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    area_id: u16,
    realtime_seconds: u64,
) -> Option<ChatCommandResult> {
    let (channel_nr, raw_text) = legacy_chat_command_channel(command)?;
    let Some(channel) = legacy_chat_channel(channel_nr) else {
        return None;
    };
    let Some(sender) = world.characters.get(&sender_id) else {
        return Some(ChatCommandResult::default());
    };
    let text = raw_text.trim_start();
    if text.is_empty() {
        return Some(ChatCommandResult {
            sender_messages: vec!["You cannot send empty chat messages.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if text.len() > 200 {
        return Some(ChatCommandResult {
            sender_messages: vec!["This chat message is too long.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    if let Some(player) = runtime.player_for_character_mut(sender_id) {
        if let Some(messages) = legacy_swearing_feedback(
            player,
            sender.flags.contains(CharacterFlags::PLAYER),
            sender.flags.contains(CharacterFlags::GOD),
            text,
            realtime_seconds,
        ) {
            return Some(ChatCommandResult {
                sender_messages: Vec::new(),
                delivered_message_bytes: messages
                    .into_iter()
                    .map(|message| (sender_id, message))
                    .collect(),
            });
        }
    }

    if sender.flags.contains(CharacterFlags::PLAYER)
        && (channel_nr == 0 || channel_nr == 32)
        && !sender.flags.contains(CharacterFlags::GOD)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && channel_nr == 31
        && !sender
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && channel_nr == 14
        && !sender
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::DEVELOPER)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let sender_runtime = runtime.player_for_character(sender_id);
    if sender.flags.contains(CharacterFlags::PLAYER) && channel_nr != 0 {
        let bit = 1_u32 << (channel_nr - 1);
        if !sender_runtime.is_some_and(|player| player.chat_channels & bit != 0) {
            return Some(ChatCommandResult {
                sender_messages: vec!["You must join a channel before you can use it.".to_string()],
                delivered_message_bytes: Vec::new(),
            });
        }
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && (channel_nr == 7 || channel_nr == 12)
        && sender.clan == 0
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied - clan members only.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER) && channel_nr == 13 {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied - club members only.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let sender_mirror = sender_runtime
        .map(|player| player.current_mirror_id)
        .unwrap_or_default();
    let staff_code = staff_code_for(Some(runtime), world, sender_id);
    let line = legacy_chat_line(sender, staff_code, sender_mirror, channel, text);
    let sender_public_id = if sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        0
    } else {
        sender.id.0
    };
    let sender_clan = sender.clan;

    let mut delivered_message_bytes = Vec::new();
    for player in runtime.players.values_mut() {
        let Some(target_id) = player.character_id else {
            continue;
        };
        let Some(target) = world.characters.get(&target_id) else {
            continue;
        };
        if channel_nr == 0 {
            delivered_message_bytes.push((target_id, line.clone()));
            continue;
        }

        let bit = 1_u32 << (channel_nr - 1);
        if player.chat_channels & bit == 0 {
            continue;
        }
        if channel_nr == 14
            && !target
                .flags
                .intersects(CharacterFlags::DEVELOPER | CharacterFlags::GOD)
        {
            player.chat_channels &= !bit;
            continue;
        }
        if channel_nr == 31
            && !target.flags.intersects(
                CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER,
            )
        {
            player.chat_channels &= !bit;
            continue;
        }
        if channel_nr == 32 && !target.flags.contains(CharacterFlags::GOD) {
            player.chat_channels &= !bit;
            continue;
        }
        if sender_public_id != 0 && player.ignores_character(sender_public_id) {
            continue;
        }
        if channel_nr == 7 && target.clan != sender_clan {
            continue;
        }
        if channel_nr == 12
            && target.clan != sender_clan
            && !world
                .clan_registry
                .relations()
                .alliance(sender_clan, target.clan)
        {
            continue;
        }
        if channel_nr == 8 && area_id == 0 {
            continue;
        }
        if channel_nr == 9 && (area_id == 0 || player.current_mirror_id != sender_mirror) {
            continue;
        }
        if channel_nr == 13 {
            continue;
        }
        delivered_message_bytes.push((target_id, line.clone()));
    }

    let spy_kind = match channel_nr {
        7 => Some("CLAN"),
        8 => Some("AREA"),
        9 => Some("MIRROR"),
        12 => Some("ALLIANCE"),
        13 => Some("CLUB"),
        _ => None,
    };
    if let Some(spy_kind) = spy_kind {
        let bit = 1_u32 << (channel_nr - 1);
        let spy_line = legacy_spy_line(spy_kind, &line);
        for player in runtime.players.values() {
            let Some(target_id) = player.character_id else {
                continue;
            };
            let Some(target) = world.characters.get(&target_id) else {
                continue;
            };
            if !target
                .flags
                .contains(CharacterFlags::GOD | CharacterFlags::SPY)
            {
                continue;
            }
            let would_see_normally = match channel_nr {
                7 => player.chat_channels & bit != 0 && target.clan == sender_clan,
                12 => {
                    player.chat_channels & bit != 0
                        && (target.clan == sender_clan
                            || world
                                .clan_registry
                                .relations()
                                .alliance(sender_clan, target.clan))
                }
                8 => player.chat_channels & bit != 0 && area_id != 0,
                9 => {
                    player.chat_channels & bit != 0
                        && area_id != 0
                        && player.current_mirror_id == sender_mirror
                }
                13 => false,
                _ => false,
            };
            if would_see_normally {
                continue;
            }
            delivered_message_bytes.push((target_id, spy_line.clone()));
        }
    }

    Some(ChatCommandResult {
        sender_messages: Vec::new(),
        delivered_message_bytes,
    })
}

pub(crate) fn apply_channels_command(command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() || !"channels".starts_with(&lower) {
        return None;
    }

    Some(KeyringCommandResult {
        messages: LEGACY_CHAT_CHANNELS
            .iter()
            .map(|channel| {
                format!(
                    "{:>2}: {:<10.10} - {}",
                    channel.number, channel.name, channel.description
                )
            })
            .collect(),
        ..Default::default()
    })
}

pub(crate) fn apply_join_leave_chat_command(
    player: &mut PlayerRuntime,
    character_flags: CharacterFlags,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.len() > "join".len() && "joinall".starts_with(&lower) {
        for nr in 1..=13 {
            player.chat_channels |= 1_u32 << (nr - 1);
        }
        return Some(KeyringCommandResult {
            messages: vec!["You have joined all channels.".to_string()],
            ..Default::default()
        });
    }

    let is_join = !lower.is_empty() && "join".starts_with(&lower);
    let is_leave = !lower.is_empty() && "leave".starts_with(&lower);
    if !is_join && !is_leave {
        return None;
    }

    let nr = legacy_atoi_prefix(rest.trim_start());
    if !(1..=32).contains(&nr) {
        return Some(KeyringCommandResult {
            messages: vec!["Channel number must be between 1 and 32.".to_string()],
            ..Default::default()
        });
    }
    let nr = nr as u8;
    let Some(channel) = legacy_chat_channel(nr) else {
        return Some(KeyringCommandResult {
            messages: vec![format!("Channel number must be between 1 and 32.")],
            ..Default::default()
        });
    };
    let bit = 1_u32 << (nr - 1);

    if is_join {
        if player.chat_channels & bit != 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "You have already joined channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        if nr == 31
            && !character_flags.intersects(
                CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER,
            )
        {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Permission denied to join channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        if nr == 32 && !character_flags.contains(CharacterFlags::GOD) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Permission denied to join channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        player.chat_channels |= bit;
        Some(KeyringCommandResult {
            messages: vec![format!(
                "You have joined channel {} ({}).",
                nr, channel.name
            )],
            ..Default::default()
        })
    } else {
        if player.chat_channels & bit == 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "You have already left channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        player.chat_channels &= !bit;
        Some(KeyringCommandResult {
            messages: vec![format!("You have left channel {} ({}).", nr, channel.name)],
            ..Default::default()
        })
    }
}

pub(crate) fn apply_clearignore_command(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("clearignore") {
        return None;
    }

    let player = runtime.player_for_character_mut(character_id)?;
    player.clear_ignored_characters();
    Some(KeyringCommandResult {
        messages: vec!["Ignore list is now empty.".to_string()],
        ..Default::default()
    })
}

pub(crate) fn apply_ignore_command(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"ignore".starts_with(&lower) {
        return None;
    }

    let rest = rest.trim_start();
    if rest.is_empty() {
        let Some(player) = runtime.player_for_character_mut(character_id) else {
            return Some(KeyringCommandResult::default());
        };
        let mut messages = Vec::new();
        player.ignored_characters.retain(|ignored_id| {
            if let Some(character) = world
                .characters
                .values()
                .find(|character| character.id.0 == *ignored_id)
            {
                messages.push(format!("Ignoring: {}", character.name));
                true
            } else {
                messages.push("Removed deleted char from list.".to_string());
                false
            }
        });
        if messages.is_empty() {
            messages.push("Ignore list is empty.".to_string());
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    let (name, _) = take_legacy_alpha_name(rest);
    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(KeyringCommandResult {
            messages: vec!["No player by that name.".to_string()],
            ..Default::default()
        });
    };
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let result = player.toggle_ignored_character(target_id.0);
    let message = match result {
        IgnoreToggleResult::Added => "Added to ignore list.",
        IgnoreToggleResult::Removed => "Deleted from ignore list.",
        IgnoreToggleResult::Full => "Ignore list is full, cannot add.",
    };
    Some(KeyringCommandResult {
        messages: vec![message.to_string()],
        ..Default::default()
    })
}

pub(crate) fn apply_tell_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
    realtime_seconds: u64,
) -> Option<TellCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("tell") {
        return None;
    }

    let rest = rest.trim_start();
    let (name, message) = take_legacy_alpha_name(rest);
    let message = message.trim_start();
    let Some(sender) = world.characters.get(&sender_id) else {
        return Some(TellCommandResult::default());
    };

    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(TellCommandResult {
            sender_messages: vec![format!("Sorry, no player by the name {name}.")],
            delivered_messages: Vec::new(),
            delivered_message_bytes: Vec::new(),
        });
    };
    let Some(target) = world.characters.get(&target_id) else {
        return Some(TellCommandResult::default());
    };
    if message.is_empty() {
        return Some(TellCommandResult {
            sender_messages: vec!["Tell, yes, tell it will be, but tell what?".to_string()],
            delivered_messages: Vec::new(),
            delivered_message_bytes: Vec::new(),
        });
    }

    if let Some(player) = runtime.player_for_character_mut(sender_id) {
        if let Some(messages) = legacy_swearing_feedback(
            player,
            sender.flags.contains(CharacterFlags::PLAYER),
            sender.flags.contains(CharacterFlags::GOD),
            message,
            realtime_seconds,
        ) {
            return Some(TellCommandResult {
                sender_messages: Vec::new(),
                delivered_messages: Vec::new(),
                delivered_message_bytes: messages
                    .into_iter()
                    .map(|message| (sender_id, message))
                    .collect(),
            });
        }
    }

    let staffmode = sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD);
    if let Some(sender_runtime) = runtime.player_for_character_mut(sender_id) {
        sender_runtime
            .tell_data
            .register_sent_tell(target.id.0, current_tick);
    }

    let mut result = TellCommandResult {
        sender_messages: vec![format!("Told {}: \"{}\"", target.name, message)],
        delivered_messages: Vec::new(),
        delivered_message_bytes: Vec::new(),
    };
    let sender_name = if sender.flags.contains(CharacterFlags::STAFF) {
        sender.name.to_ascii_uppercase()
    } else {
        sender.name.clone()
    };
    let mirror = runtime
        .player_for_character(sender_id)
        .map(|player| player.current_mirror_id)
        .unwrap_or_default();
    let staff_code = if sender.flags.contains(CharacterFlags::STAFF) {
        format!(" [{}]", staff_code_for(Some(runtime), world, sender_id))
    } else {
        String::new()
    };
    let tell_text = format!(
        "{}{} ({}) tells you: \"{}\"",
        sender_name, staff_code, mirror, message
    );
    let tell_payload = tell_text.as_bytes().to_vec();
    let spy_line = legacy_spy_line("TELL", &tell_payload);
    for player in runtime.players.values() {
        let Some(spy_character_id) = player.character_id else {
            continue;
        };
        if spy_character_id == sender_id || spy_character_id == target_id {
            continue;
        }
        let Some(spy_character) = world.characters.get(&spy_character_id) else {
            continue;
        };
        if spy_character
            .flags
            .contains(CharacterFlags::GOD | CharacterFlags::SPY)
        {
            result
                .delivered_message_bytes
                .push((spy_character_id, spy_line.clone()));
        }
    }

    if target.flags.contains(CharacterFlags::NOTELL) && !staffmode {
        return Some(result);
    }
    if !staffmode
        && runtime
            .player_for_character(target_id)
            .is_some_and(|player| player.ignores_character(sender.id.0))
    {
        return Some(result);
    }
    result.delivered_messages.push((target_id, tell_text));

    if let Some(target_runtime) = runtime.player_for_character_mut(target_id) {
        target_runtime.tell_data.register_received_tell(sender.id.0);
    }
    if target_id == sender_id {
        result
            .sender_messages
            .push("Do you like talking to yourself?".to_string());
    }

    Some(result)
}

pub(crate) fn drain_expired_tell_feedback(
    world: &World,
    runtime: &mut ServerRuntime,
    current_tick: u64,
) -> Vec<(CharacterId, Vec<u8>)> {
    let mut feedback = Vec::new();
    for player in runtime.players.values_mut() {
        let Some(character_id) = player.character_id else {
            continue;
        };
        for target_id in player.tell_data.check_tells(current_tick, TICKS_PER_SECOND) {
            let name = world
                .characters
                .values()
                .find(|character| character.id.0 == target_id)
                .map(|character| character.name.as_str())
                .unwrap_or("Someone");
            feedback.push((character_id, tell_not_listening_message(name)));
        }
    }
    feedback
}

pub(crate) fn apply_nowho_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower != "nowho" {
        return None;
    }

    let character = world.characters.get_mut(&character_id)?;
    if !character
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return None;
    }

    character.flags.toggle(CharacterFlags::NOWHO);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "NoWho {}.",
            if character.flags.contains(CharacterFlags::NOWHO) {
                "enabled"
            } else {
                "disabled"
            }
        )],
        ..Default::default()
    })
}

pub(crate) fn apply_who_command(
    world: &World,
    runtime: Option<&ServerRuntime>,
    caller_flags: CharacterFlags,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() >= 4
        && "whostaff".starts_with(&lower)
        && caller_flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        let mut characters = world.characters.values().collect::<Vec<_>>();
        characters.sort_by_key(|character| character.id.0);

        let mut messages = Vec::new();
        for character in characters {
            if character.flags.contains(CharacterFlags::INVISIBLE) {
                continue;
            }
            if !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if !character
                .flags
                .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
            {
                continue;
            }

            messages.push(format!(
                "{} [{}]{}",
                character.name,
                staff_code_for(runtime, world, character.id),
                if character.driver == 0 {
                    ""
                } else {
                    " (lagging)"
                }
            ));
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.is_empty() || !"who".starts_with(&lower) {
        return None;
    }

    let mut characters = world.characters.values().collect::<Vec<_>>();
    characters.sort_by_key(|character| character.id.0);

    let mut messages = vec!["Currently online in this area:".to_string()];
    for character in characters {
        if character.flags.contains(CharacterFlags::INVISIBLE) {
            continue;
        }
        if !character.flags.contains(CharacterFlags::PLAYER) {
            continue;
        }
        if character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
            && character.flags.contains(CharacterFlags::NOWHO)
        {
            continue;
        }

        let arch = if character.flags.contains(CharacterFlags::ARCH) {
            "A"
        } else {
            ""
        };
        let warrior = if character.flags.contains(CharacterFlags::WARRIOR) {
            "W"
        } else {
            ""
        };
        let mage = if character.flags.contains(CharacterFlags::MAGE) {
            "M"
        } else {
            ""
        };
        messages.push(format!(
            "{} ({}{}{}{})",
            character.name, arch, warrior, mage, character.level
        ));
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}
