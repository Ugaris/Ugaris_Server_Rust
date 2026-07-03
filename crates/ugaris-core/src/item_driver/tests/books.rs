use super::*;

#[test]
fn book_driver_returns_legacy_text_kind() {
    let mut character = character(1);
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOK);
    set_drdata(&mut book, 0, 8);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BOOK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BOOK, 16);
    assert_eq!(
        execute_item_driver(&mut character, &mut book, request, 2, false),
        ItemDriverOutcome::BookText {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            kind: 8,
            demon_value: 0,
        }
    );
    assert_eq!(
        book_text_lines(8)[0],
        "There are two kinds of vampires. One is known under varying names, such as 'Vampire', 'Lesser Vampire', 'Dracul' or 'Necrifah'."
    );
}

#[test]
fn book_driver_ignores_timer_style_zero_character_calls() {
    let mut character = character(0);
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOK);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BOOK,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut book, request, 2, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn book_header_constants_match_legacy_values() {
    assert_eq!(BOOK_LOISAN1, 0);
    assert_eq!(BOOK_VAMPIRE5, 12);
    assert_eq!(BOOK_DEMON1, 13);
    assert_eq!(BOOK_DEMON5, 17);
    assert_eq!(SIGN_EDEMON1, 18);
    assert_eq!(SIGN_EDEMON2, 19);
    assert_eq!(BOOK_EDEMON3, 22);
    assert_eq!(BOOK_IDEMON1, 24);
    assert_eq!(BOOK_RUNES1, 31);
    assert_eq!(BOOK_RUNES8, 38);
    assert_eq!(BOOK_BONES1, 39);
    assert_eq!(BOOK_SHRIKE, 42);
    assert_eq!(BOOK_GWENDYLON, 43);
    assert_eq!(BOOK_MADMAGES_BOOK1, 44);
    assert_eq!(SIGN_FOREST_ARENA, 46);
    assert_eq!(SIGN_ARENA, 47);
    assert_eq!(BOOK_NOOK_JOKES, 48);
    assert_eq!(BOOK_LAB2_DIARY, 100);
    assert_eq!(BOOK_LAB2_DIARY_PAGE, 101);
}

#[test]
fn book_text_lines_include_static_later_legacy_books() {
    assert_eq!(
        book_text_lines(BOOK_IDEMON1)[0],
        "Day 155, year 103. Personal diary of Kamaleon of the Isara."
    );
    assert_eq!(
        book_text_lines(BOOK_PALACE3)[5],
        "The cold is slowly killing all of us. All attempts to control the demon lords have failed. Now all of us must die. But I shall die happily if I can take Ishtar with me into the cold."
    );
    assert_eq!(
        book_text_lines(BOOK_RUNES8),
        &["Berkano, Ehwaz, Ansuz will decrease magic damage."][..]
    );
    assert_eq!(
        book_text_lines(BOOK_LAB2_DIARY)[2],
        "The last fight with the undeads was hard. But even though I am bleeding from many wounds, today is the day I will kill my brother. I will take the amulet and go into the family vault and face him now!"
    );
}

#[test]
fn book_text_lines_include_raw_color_marker_book_cases() {
    assert_eq!(
        book_text_lines(BOOK_RUNES1)[0],
        "Personal Diary of Korzam, Magical Advisor of Scarcewind."
    );
    assert_eq!(book_text_lines(BOOK_BONES1)[2], "You skip some pages.");
    assert_eq!(
        book_text_lines(BOOK_GWENDYLON),
        &["Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles."][..]
    );
    assert_eq!(
        book_text_lines(BOOK_MADMAGES_BOOK1)[2],
        "At the bottom of the following page you find a list of the current teachers of the mages order: Bretl, Anna-Sofia, Leaner, Crem, Guiwynn."
    );

    let runes = book_text_line_bytes(BOOK_RUNES1);
    assert_eq!(&runes[1][..3], crate::text::COL_DARK_GRAY);
    assert!(runes[1].ends_with(b"replaced by:"));

    let mad_mages = book_text_line_bytes(BOOK_MADMAGES_BOOK1);
    assert_eq!(&mad_mages[2][..3], crate::text::COL_DARK_GRAY);
    assert!(mad_mages[2]
        .windows(3)
        .any(|bytes| bytes == crate::text::COL_RESET));
    assert!(mad_mages[2].ends_with(b"Bretl, Anna-Sofia, Leaner, Crem, Guiwynn."));
}

#[test]
fn earth_demon_sign_books_use_reader_demon_knowledge() {
    assert_eq!(
        book_text_line_bytes_for_reader(SIGN_EDEMON1, 0),
        vec![b"It's written in strange letters you cannot read.".to_vec()]
    );
    assert_eq!(
        book_text_line_bytes_for_reader(SIGN_EDEMON2, 1),
        vec![b"You recognice some of the letters used in this sign from your studies of the ancient knowledge, but you cannot tell what the sign means.".to_vec()]
    );
    assert_eq!(
        book_text_line_bytes_for_reader(SIGN_EDEMON1, 2),
        vec![b"Defense Systems Control Room".to_vec()]
    );
    assert_eq!(
        book_text_line_bytes_for_reader(SIGN_EDEMON2, 2),
        vec![
            b"Research Laboratorium".to_vec(),
            b"Caution, live demons!".to_vec(),
        ]
    );
}

#[test]
fn demon_books_generate_legacy_character_specific_ritual_words() {
    assert_eq!(demon_ritual_words(6, 2), "shirsli sausgadul");
    assert_eq!(
        book_text_line_bytes_for_reader_id(BOOK_DEMON3, 0, 6),
        vec![b"'shirsli sausgadul' will give thee even better protection.".to_vec()]
    );
    assert_eq!(
        book_text_line_bytes_for_reader_id(BOOK_DEMON1, 0, 6),
        vec![b"I have seen in written in fiery letters upon the sky: Those who have the knowledge can invoke protection against demonic might by uttering the words: 'dorsli kilaghshir'".to_vec()]
    );
    assert_ne!(
        book_text_line_bytes_for_reader_id(BOOK_DEMON1, 0, 6),
        book_text_line_bytes_for_reader_id(BOOK_DEMON1, 0, 7)
    );
}

#[test]
fn book_nook_joke_lines_match_legacy_random_cases() {
    assert_eq!(
        book_nook_joke_line_bytes(0),
        vec![
            b"What did the fisherman say to the card magician?".to_vec(),
            b"Pick a cod, any cod!".to_vec(),
        ]
    );
    assert_eq!(
        book_nook_joke_line_bytes(4),
        vec![
            b"What bone will a dog never eat?".to_vec(),
            b"A trombone.".to_vec(),
        ]
    );
    assert_eq!(book_nook_joke_line_bytes(9), book_nook_joke_line_bytes(4));
}

#[test]
fn book_special_effects_match_legacy_earth_demon_diaries() {
    assert_eq!(book_special_effect(BOOK_EDEMON3), Some(50287));
    assert_eq!(book_special_effect(BOOK_EDEMON4), Some(50305));
    assert_eq!(book_special_effect(BOOK_EDEMON1), None);
    assert_eq!(book_special_effect(BOOK_IDEMON1), None);
}
