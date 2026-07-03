use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaToplistEntry {
    pub name: String,
    pub score: i32,
}

pub fn arena_toplist_lines(
    entries: &[ArenaToplistEntry],
    score: i32,
    wins: i32,
    losses: i32,
    fights: i32,
) -> Vec<String> {
    let player_score = if fights == 0 { -2000 } else { score };
    let mut lines = Vec::new();

    for (index, entry) in entries.iter().take(10).enumerate() {
        if entry.name.is_empty() {
            break;
        }
        lines.push(format!("{}: {} {}", index + 1, entry.name, entry.score));
    }

    let mut rank_index = 10usize;
    while rank_index < entries.len().min(100) {
        let entry = &entries[rank_index];
        if entry.name.is_empty() || entry.score < player_score {
            break;
        }
        rank_index += 1;
    }

    let start = rank_index.saturating_sub(5).max(10);
    let end = (rank_index + 5).min(entries.len()).min(100);
    for index in start..end {
        let entry = &entries[index];
        if entry.name.is_empty() {
            break;
        }
        lines.push(format!("{}: {} {}", index + 1, entry.name, entry.score));
    }

    lines.push(format!(
        "Your score is {player_score}, you have won {wins} fights and lost {losses} fights."
    ));
    lines
}

pub(crate) fn toplist_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ArenaToplist {
        item_id: item.id,
        character_id: character.id,
    }
}
