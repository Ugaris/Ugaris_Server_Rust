use super::*;

pub(crate) fn apply_status_command(
    character: &Character,
    player: &PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() || !"status".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![
        "Lag Control Settings:".to_string(),
        format!("Max. Lag [/MAXLAG]: {} sec.", player.max_lag_seconds),
    ];

    let on_off = |flag: bool| if flag { "On" } else { "Off" };
    let has_spell = |value: CharacterValue| character.values[1][value as usize] > 0;
    if has_spell(CharacterValue::Flash) {
        messages.push(format!(
            "Don't use Ball Lightning [/NOBALL]: {}.",
            on_off(player.no_ball)
        ));
    }
    if has_spell(CharacterValue::Bless) {
        messages.push(format!(
            "Don't use Bless [/NOBLESS]: {}.",
            on_off(player.no_bless)
        ));
    }
    if has_spell(CharacterValue::Fireball) {
        messages.push(format!(
            "Don't use Fireball [/NOFIREBALL]: {}.",
            on_off(player.no_fireball)
        ));
    }
    if has_spell(CharacterValue::Flash) {
        messages.push(format!(
            "Don't use Lightning Flash [/NOFLASH]: {}.",
            on_off(player.no_flash)
        ));
    }
    if has_spell(CharacterValue::Freeze) {
        messages.push(format!(
            "Don't use Freeze [/NOFREEZE]: {}.",
            on_off(player.no_freeze)
        ));
    }
    if has_spell(CharacterValue::Heal) {
        messages.push(format!(
            "Don't use Heal [/NOHEAL]: {}.",
            on_off(player.no_heal)
        ));
    }
    if has_spell(CharacterValue::MagicShield) {
        messages.push(format!(
            "Don't use Magic Shield [/NOSHIELD]: {}.",
            on_off(player.no_shield)
        ));
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push(format!(
            "Don't use Pulse [/NOPULSE]: {}.",
            on_off(player.no_pulse)
        ));
    }
    if has_spell(CharacterValue::Warcry) {
        messages.push(format!(
            "Don't use Warcry [/NOWARCRY]: {}.",
            on_off(player.no_warcry)
        ));
    }

    messages.extend([
        format!(
            "Don't use Healing Potions [/NOLIFE]: {}.",
            on_off(player.no_life)
        ),
        format!(
            "Don't use Mana Potions [/NOMANA]: {}.",
            on_off(player.no_mana)
        ),
        format!(
            "Don't use Combo Potions [/NOCOMBO]: {}.",
            on_off(player.no_combo)
        ),
        format!(
            "Don't use Recall Scroll [/NORECALL]: {}.",
            on_off(player.no_recall)
        ),
        format!("Don't Move [/NOMOVE]: {}.", on_off(player.no_move)),
        "Automation Settings:".to_string(),
    ]);
    if has_spell(CharacterValue::Bless) {
        messages.push(format!(
            "Automatic Re-Bless [/AUTOBLESS]: {}.",
            on_off(player.autobless_enabled)
        ));
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push(format!(
            "Automatic Pulse [/AUTOPULSE]: {}.",
            on_off(player.autopulse_enabled)
        ));
    }
    messages.extend([
        format!(
            "Automatic Turning [/AUTOTURN]: {}.",
            if player.autoturn_enabled { "On" } else { "Off" }
        ),
        "Protection Settings:".to_string(),
        format!(
            "Allow others to bless me [/ALLOWBLESS]: {}.",
            if character.flags.contains(CharacterFlags::NOBLESS) {
                "No"
            } else {
                "Yes"
            }
        ),
        "Account Status:".to_string(),
        if character.flags.contains(CharacterFlags::PAID) {
            "Paid Account".to_string()
        } else {
            "Trial Account".to_string()
        },
    ]);

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}
