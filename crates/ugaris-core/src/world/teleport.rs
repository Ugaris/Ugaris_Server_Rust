use super::*;

impl World {
    pub(crate) fn consume_city_recall_scroll(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
    ) {
        let Some(item) = self.items.get_mut(&item_id) else {
            return;
        };
        item.driver_data.resize(2, 0);
        if item.driver_data[1] > 1 {
            item.driver_data[1] -= 1;
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
            return;
        }

        if let (Some(character), Some(item)) = (
            self.characters.get_mut(&character_id),
            self.items.get_mut(&item_id),
        ) {
            consume_item(character, item);
        }
    }

    pub(crate) fn teleport_character_exact(
        &mut self,
        character_id: CharacterId,
        x: usize,
        y: usize,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        let before = character.clone();
        remove_character_light(&mut self.map, character);
        self.map.remove_char(character);
        character.action = 0;
        character.step = 0;
        character.duration = 0;
        if !self.map.set_char(character, x, y) {
            let _ = self.map.set_char(character, old_x, old_y);
            add_character_light(&mut self.map, character);
            let after = character.clone();
            self.mark_character_light_area(&before);
            self.mark_character_light_area(&after);
            return false;
        }
        add_character_light(&mut self.map, character);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
        true
    }

    pub(crate) fn teleport_character(
        &mut self,
        character_id: CharacterId,
        x: u16,
        y: u16,
        extended: bool,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        let before = character.clone();
        remove_character_light(&mut self.map, character);
        self.map.remove_char(character);
        character.action = 0;
        character.step = 0;
        character.duration = 0;
        let placed = if extended {
            self.map
                .drop_char_extended(character, usize::from(x), usize::from(y), 6)
        } else {
            self.map
                .drop_char(character, usize::from(x), usize::from(y))
        };
        if !placed {
            let _ = self.map.drop_char(character, old_x, old_y);
            add_character_light(&mut self.map, character);
            let after = character.clone();
            self.mark_character_light_area(&before);
            self.mark_character_light_area(&after);
            return false;
        }
        add_character_light(&mut self.map, character);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
        true
    }

    pub fn teleport_character_same_area(
        &mut self,
        character_id: CharacterId,
        x: u16,
        y: u16,
        extended: bool,
    ) -> bool {
        self.teleport_character(character_id, x, y, extended)
    }

    /// C `teleport_char_driver` (`src/system/drvlib.c:2651-2673`): a no-op
    /// when already within Manhattan distance `1` of the target, otherwise
    /// remove-and-redrop trying the exact tile then its 8 neighbors (C's
    /// `drop_char`, matching [`World::teleport_character`]'s non-extended
    /// mode), falling back to the old position if every candidate tile is
    /// blocked/occupied. Returns whether the character actually moved.
    pub fn teleport_char_driver(&mut self, character_id: CharacterId, x: u16, y: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let dx = i32::from(character.x) - i32::from(x);
        let dy = i32::from(character.y) - i32::from(y);
        if dx.abs() + dy.abs() < 2 {
            return false;
        }
        self.teleport_character(character_id, x, y, false)
    }
}
