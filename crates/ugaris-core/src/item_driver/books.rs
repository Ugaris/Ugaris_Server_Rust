use super::*;

pub fn book_text_lines(kind: u8) -> &'static [&'static str] {
    match kind {
        0 => &[
            "The magical properties of these skulls are astonishing. They are the artifacts the various shrines of the ancients accept, they can also be used to animate skeletons.",
            "After I told Moakin about them he used some magical stones to enhance the skulls and he created a small army of skeletons. Too bad his hunger for power made him go away without sharing his secrets with me.",
            "I wonder what became of him, and his puppet, Dorugin. But I digress. Now that Moakin has left, I will have to find out how to control the undead created with these skulls.",
            "My experiments have been successful in raising skeletons and zombies. A single plain skull can be used to create about a dozen of them. I wonder what one of the rare silver skulls would do.",
            "But I still have to control my creations. How Moakin managed to do that escapes me. I have tried various potions on those fools in Cameron to understand how to control a mind, but to no avail.",
            "It seems alchemy is worthless when it comes to control. I will have to resort to magical jewels. But those are hard to find. Maybe one of the shrines will produce some?",
        ],
        1 => &[
            "Healing Potions. Mana Potions. Torches. Small magical effects. Plain skulls are worthless. Must try the silver ones.",
            "Ahh. These zombies are dangerous. But they shall not stop me, Loisan. No luck with silver skulls either.",
            "Must try to get golden ones. But the danger...",
            "I am dying. So close. Oh, how cruel.",
        ],
        2 => &[
            "Day 122, year 48, morning by outside time. Personal diary of Ioslan of the Cerasa.",
            "We had to retreat further into the tunnels. The enemy is sending a new type of monster. Our creatures fight valiantly, but they cannot withstand them for long. We will have to flee, or we will perish.",
            "Armenicon has created more powerful creatures, but they fail to recognize us. Therefore, Armenicon added keywords to them, which will stun them for a short time, allowing us to flee from them.",
            "Once they are released, we will leave this part of the tunnel system and hope our enemy will invade and die. Today it is my turn to sneak through the tunnels and collect the skulls of our creatures.",
            "The enemy still has not learned the value of them. I just hope I will survive to flee with my kin.",
        ],
        3 => &[
            "Specimen 33. Prototype 4. Keyword: Nazimah.",
            "I will send this creature to guard the huge cavern. We cannot prevent the enemy from taking our storage room, but we can make him pay dearly for it.",
        ],
        4 => &[
            "Specimen 35. Prototype 4. Keyword: Argatoth.",
            "Another guard for our storage room.",
        ],
        5 => &[
            "Day 122, year 48, evening by outside time. Personal diary of Armenicon of the Cerasa.",
            "Ioslan has not returned. We cannot tell if he managed to recharge the spawners or not. We must flee immediately. The enemy will attack very soon.",
        ],
        6 => &[
            "Specimen 34. Prototype 4. Keyword: Lorganoth.",
            "Good. Prototype 4 is very difficult to create, but extremely powerful. This creature is to guard the storage room.",
        ],
        7 => &[
            "Specimen 36. Prototype 4. Keyword: Markanoth.",
            "The last prototype 4 for the storage room. These creatures are deadly.",
        ],
        8 => &[
            "There are two kinds of vampires. One is known under varying names, such as 'Vampire', 'Lesser Vampire', 'Dracul' or 'Necrifah'.",
            "Of the other kind, only a few sources report. They are called 'Vampire Lords' or 'Methusalah'.",
            "Killing a Lesser Vampire is as simple as penetrating it with a sword, or frying it with magic. They possess the abilities of the human they were once, but not much more.",
            "But killing a Vampire Lord on the other hand is very difficult, since each of them only has one weakness. Discovering that weakness is of utmost importance.",
            "Even if the weakness is known, it will still be a hard battle, as Vampire Lords are extremely old and powerful.",
        ],
        9 => &[
            "In a vision, I saw a sun shine in the darkness, and I saw fear in the eyes of the Lord.",
            "But then the sun was shattered, and parts of it fell into the dark. The Lord took them, and hid them in His lair.",
            "Then I saw Him leave His crypt, and come for me.",
        ],
        10 => &["One among many, one pointing sideways, part you shall find there. Cross I shall be with thee, shouldst thou fail."],
        11 => &[
            "'And,' said the wise, 'If ye are burning, my pupil, what shall ye do?'",
            "'Extinguish the flames, master?'",
        ],
        12 => &[
            "Take heed, and go no further! This way leads to the Vampire Lord!",
            "It is said that one strike with the right dagger will kill the Lord. But alas, many have tried, but no one found the right dagger.",
        ],
        20 => &[
            "Day 91, year 97, evening by outside time. Personal diary of Avaisor of the Isara.",
            "The struggle seems hopeless now. We're trapped in these caverns by our own defense systems. We can no longer control them as the key was lost when Daoslan was slain by demons in the southern part of the natural caverns.",
            "Our desperate attempts to raise demons for our defense have failed so far. Some of the research labs had to be closed since the demons in them could no longer be controlled.",
        ],
        21 => &[
            "Day 58, year 97, memo on the state of War by Seraios of the Isara",
            "Only one adversary remains after the glorious defeat of Keriaos. But it is a dangerous one. Islena has persuaded four of our enemies to join forces with her, and she will gather all her allies in the north to form an army capable of destroying us.",
            "We must make our move first and attack before she is ready. I advise that we attack Islena's headquarter with...",
            "(The remaining pages are burned.)",
        ],
        22 => &[
            "Day 84, year 97, evening by outside time. Personal diary of Delasar of the Isara.",
            "I have established two outposts beyond our defense line to the north. They will allow me to study the demons as they are attacking our defense systems. I might be able to find other means to protect us this way.",
            "Going there is dangerous, and I might not make it back with my knowledge. I will keep other diaries there, so that my clan will be able to use my findings even after my death.",
            "I have asked Avaisor to turn off the defense systems in an hour so that I can reach my outpost. Fate, let me survive!",
        ],
        23 => &[
            "Day 55, year 97, evening by outside time. Personal diary of Isranor of the Cerasa.",
            "Our glorious leader, Carisar, has joined forces with Islena of the Ilasner. Our talks with our direct enemy the Isara have failed. Too much blood was shed already and neither they nor we could overcome the hatred. But still, I was impressed by Ishtar, their leader.",
            "In spite of our alliance with the Ilasner, our position here is quite hopeless. The Isara will soon be forced to attack with all their might. Our defenses cannot withstand them for long. We will abandon this position within the next few days.",
            "Fortunately, we made good progress with our demonic research project. We will not suffer much from the demons that escaped during the early stages of our research. And we can hope that they will delay the Isara's pursuit.",
            "We will open the demon-gate before we leave. The steady flood of demons coming from it will give us the time we need and hinder the Isara.",
        ],
        24 => &[
            "Day 155, year 103. Personal diary of Kamaleon of the Isara.",
            "In our pursuit of Islena's forces we have finally reached one of her former settlements. The long march north, first through that fiery maze full of lava and unmanned yet dangerous defense stations and now through these icy caverns has tired us beyond measure.",
            "So many friends lost, so many deaths. And yet we must press on after only a few days rest, lest we give Islena time to counter-attack and crush us while we are defenseless. I wonder how far this pursuit will take us. We have come so far north in these caverns, we must be below the sea already.",
            "But we will not stop, it would mean death. Mind, be tranquil, this will end.",
        ],
        25 => &[
            "Day 158, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "The three days rest we have given our men are all the time we can spare. Not all wounds are healed, and the men are still tired, but delaying further would leave us open to a counter-attack. I wonder what the Ilasner are up to. It is not like them to give up this much ground without any resistance.",
            "Tomorrow at dawn, well, tomorrow when we wake up, we will move on. We still have some wood to build fires to break the ice demon's spell, and the morale is as well as can be expected under these circumstances. I am greatly worried, though. We haven't seen the surface for years, and all the explosions we heard a few weeks ago mean the war is raging there as savage as it is here.",
        ],
        26 => &[
            "Day 145, year 103. Personal diary of Cari-Maar of the Ilasner.",
            "Today we were ordered to retreat. The defense stations and the fire demons have delayed our pursuers long enough. Rumor has it that Islena and the main force have established a defensive position further to the north.",
            "But our scouts report that those cursed Isara have managed to bring half their forces through alive. We are vastly outnumbered. We can only hope that the fortified positions will give us enough advantage to make up for our lack in numbers.",
        ],
        27 => &[
            "Contrary to my original belief, the swamp beasts possess no intelligence. The buildings they inhabitate must have been built by a now extinct people. I assume that the three stone circles have been built by the same people.",
            "Some pages later: I have discovered old drawings, showing humans fighting against swamp beasts. In the first pictures the humans flee from a huge beast. A bit further down, one of the drawings shows a human warrior standing in the center of a stone circle, holding a weapon in his hand. Strangely, it shows the sun being exactly below the warrior and the ground. The warrior seems to be waiting, and looking at the sun. In the next drawing, he is still standing in the stone circle, but now he is killing a small swamp beast. His weapon seems to be glowing.",
        ],
        28 => &[
            "Day 172, year 103. Personal diary of Cari-Maar of the Ilasner.",
            "Today we finished raising the demon lord for the trap we've built for the Isara. Let them come now, they are doomed.",
        ],
        29 => &[
            "Day 175, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "Dead. All dead. Only Ishtar and I survived the storm of demon lords the Ilasner raised. We could flee, but we are locked into these rooms. The demon lords cannot enter, but they have begun to invoke the icy cold. We will freeze to death.",
            "Day 177, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "The cold is creeping into my bones. Ishtar has kept us alive so far, but now he is exhausted and cannot sustain the heating spell. I think the whole palace is frozen.",
            "Why, oh why did we have to fight this war? The world was so beautiful, and so were we. But now, all that remains is blood and tears. If anyone survives this folly, let our fate teach you not to repeat our mistakes!",
        ],
        30 => &[
            "Day 175, year 103. Personal diary of Islena.",
            "The cursed Isara are caught in our trap. Now they will die, all of them will die.",
            "Day 176, year 103. Personal diary of Islena.",
            "It seems some of them got away. The demon lords are out of control and trying to freeze them to death. I can feel the cold even here, in my rooms.",
            "Day 177, year 103. Personal diary of Islena.",
            "The cold is slowly killing all of us. All attempts to control the demon lords have failed. Now all of us must die. But I shall die happily if I can take Ishtar with me into the cold.",
        ],
        31 => &[
            "Personal Diary of Korzam, Magical Advisor of Scarcewind.",
            "The line above has been nearly scratched out, and replaced by:",
            "Personal Diary of Korzam, Governor of Exkordon.",
            "Scarcewind, the fool, is still loyal to Aston. He does not understand that the only way for our city to prosper is to cut our ties to that rotten empire. What good is an advisor, if no one listens to him?",
            "To get my mind on other things, I have gone north, into the barren lands below the mountains, hunting rumors. It is said that huge towers are build on those plains, and in those mountains. Towers built by powerful wizards of the old age. Whoever started these rumors has his history wrong, that is for sure. There was no old age. Before us were the ancients. They destroyed each other, and the world, in their foolish war. After them came we, and Ishtar and his notions of godhood and the empire.",
            "But if these towers are really there, and if they are as magical as the rumors say, who built them? Who else but the ancients! There was no one else who could have built them. And if the ancients are the makers, those towers are old and must have survived the destructions of the war. I want to see what kind of magic can make buildings survive what has shattered the earth.",
            "You skip several pages containing a description of the voyage to the towers.",
            "I have forced my way into one of the towers. Magical they are, for sure, and guarded by the living dead. Fighting my way inside nearly exhausted me, and all I could do was grab some parchments and a small bag and flee, before those undead came back in greater numbers.",
            "The book is written in the language of the ancients. Unfortunately, I can barely understand some words. The bag contained polished pieces of bone, each bearing a rune. I will return to Exkordon now, and study them at my leisure.",
            "I found some pictures in the book, showing how to arrange the runes. I wonder what will happen...",
            "You notice a change in the writing. It is the same hand, but the letters are bigger, and more forcefully written.",
            "That does it. Scarewind is a weak fool. I shall kill him, and take Exkordons fate into my own hands.",
            "Easy, almost too easy it was. I am now Governor of Exkordon. Scarcewind died like the fool he was in life. 'How can you do that? Why? I trusted you!' What a fool. I invited him into my house, told him about an important discovery I made. He came, and left his guards outside. And so he died. When his guards came looking for him, I lured them into my cellars, and disposed of them. They are no match for the ancient's magic.",
            "Here, the writing changes back to the style used in the beginning.",
            "What have I done? What came over me? And why are the dead rising, and walking my halls? They are dead! Dead! I killed them!",
        ],
        32 => &["Once leads on, twice is rewarding, three times is dangerous."],
        33 => &["Two Berkano flanking an Ansuz will give thee Endurance."],
        34 => &["Berkano, Dagaz, Ansuz is healthy."],
        35 => &["Ansuz and Dagaz twice - good for Mages."],
        36 => &["Ansuz, Ehwaz, Dagaz - better defense for the Warrior."],
        37 => &["Ehwaz twice followed by Berkano - better defense for the Mage."],
        38 => &["Berkano, Ehwaz, Ansuz will decrease magic damage."],
        39 => &[
            "Day 12, year 45. Personal diary of Sluiran of the Caremar.",
            "The battles raging outside are closer to our hiding place. We must find some means to defend ourselves. I have started to study the forbidden art of necromancy, based on the rune magic. The undead shall fight where the living cannot.",
            "You skip some pages.",
            "Day 37, year 47. Personal diary of Sluiran of the Caremar.",
            "The towers have fallen, but the undead have held our halls against the first wave of attackers. I have many, many bodies for my work now. More and more undead shall defend us. We might survive, after all.",
        ],
        40 => &[
            "Day 213, year 61. Personal diary of Sluiran of the Caremar.",
            "We have been attacked by demons again, and we are running out of dead bodies to raise in our defense. We can no longer reach those in the outer halls. It will not take long before they take our last defenses. But they shall not gain any profit by this. I shall cast a spell that will raise all dead in these halls over and over again. So we will continue the fight, even after we are dead.",
        ],
        41 => &[
            "My dear Sarkilar,",
            "thine shall be the land from rotten Exkordon to the icy shores Valkyries. It is ripe, ready for thee to take it. The magic of the Kir should give thee sufficient strength. Take as many of the young monks as thou canst, and cloud their mind, as I taught thee. Once thy force is strong enough, take the land which is promised thee.",
            "Islena",
        ],
        42 => &["My wounds are too much to bear and I fear that I will not survive. I have found none of the parts of the Talisman of the Moon, nor the location of the Moon Pool in which to enchant it. I have failed to find a way to lift the curse off my old friend, and I am sorry."],
        43 => &["Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles."],
        44 => &[
            "It is a long list of names, the masters and teachers of the mages order. With deep respect, the great past masters of the mages order, are here honoured.",
            "Wijn, the old one. Gree-Dli, master of summoning spells, Leerea, the empat, Djurna bridgecaller, Friize the recluse, Loisan creator. ",
            "At the bottom of the following page you find a list of the current teachers of the mages order: Bretl, Anna-Sofia, Leaner, Crem, Guiwynn.",
            "It appears that someone has attempted to scratch away the final name from the parchment.",
        ],
        45 => &[
            "Sacred potions",
            "There are rumors saying a potion can be created, which holds the insignia of the very Ishtar himself. Bestowing his blessing upon the user. Imagine! The potency of such a liquid! Some of the ingredients are obvious.",
            "Sulphur for preserving power in bottled form. Some kind of transformative agent must be added, how else can mere mortals consume the insignia without being entirely overcome by it? Madness it is to directly consume such an element. And madness will be the curse upon those who attempt it.",
            "Here art no choice but to explore by testing the potions out. To balance the splendor of Ishtar's insignia it must contain a liquid harmful to humans. A poison or venom most likely. Possibly from a mushroom.",
            "My first attempt ready now. The coloration looks promising, and the volunteers are ready. This will either be a splendid achievement worthy a record in the great library of Exkordon, or a good reason for me to go under ground.",
        ],
        46 => &["This is an arena. Death on the sand incurs none of the usual penalties of death. Thou shall not loose saves, experience, equipment or gold"],
        100 => &[
            "The pages are badly burned. You can only read: All those heros who tried to kill my brother died through his hands. To keep these young hotheads away, I summoned a demon to guard the entrance and ordered him to let no one pass but me. He is a bit short-sighted, but...",
            "My brother must be killed, or the horror will never stop. He is my brother, but he must die for his misdeeds...",
            "The last fight with the undeads was hard. But even though I am bleeding from many wounds, today is the day I will kill my brother. I will take the amulet and go into the family vault and face him now!",
        ],
        101 => &["Most of the page is burned, but you can read: To prevent holy water from hurting him, and his minions, my brother created a anti-magic zone which dispells all holy effects and all magic. But I have found a way to break this spell. I created an amulet to hold the counter-spell..."],
        _ => &[],
    }
}

pub fn book_text_line_bytes(kind: u8) -> Vec<Vec<u8>> {
    book_text_line_bytes_for_reader(kind, 0)
}

pub fn book_text_line_bytes_for_reader(kind: u8, demon_value: i32) -> Vec<Vec<u8>> {
    book_text_line_bytes_for_reader_id(kind, demon_value, 0)
}

pub fn book_text_line_bytes_for_reader_id(
    kind: u8,
    demon_value: i32,
    reader_id: u32,
) -> Vec<Vec<u8>> {
    match kind {
        BOOK_DEMON1..=BOOK_DEMON5 => demon_book_line_bytes(kind, reader_id),
        SIGN_EDEMON1 => edemon_sign_line_bytes(demon_value, &["Defense Systems Control Room"]),
        SIGN_EDEMON2 => edemon_sign_line_bytes(
            demon_value,
            &["Research Laboratorium", "Caution, live demons!"],
        ),
        BOOK_RUNES1 => vec![
            plain_book_line_bytes("Personal Diary of Korzam, Magical Advisor of Scarcewind."),
            dark_gray_book_line_bytes("The line above has been nearly scratched out, and replaced by:", false),
            plain_book_line_bytes("Personal Diary of Korzam, Governor of Exkordon."),
            plain_book_line_bytes("Scarcewind, the fool, is still loyal to Aston. He does not understand that the only way for our city to prosper is to cut our ties to that rotten empire. What good is an advisor, if no one listens to him?"),
            plain_book_line_bytes("To get my mind on other things, I have gone north, into the barren lands below the mountains, hunting rumors. It is said that huge towers are build on those plains, and in those mountains. Towers built by powerful wizards of the old age. Whoever started these rumors has his history wrong, that is for sure. There was no old age. Before us were the ancients. They destroyed each other, and the world, in their foolish war. After them came we, and Ishtar and his notions of godhood and the empire."),
            plain_book_line_bytes("But if these towers are really there, and if they are as magical as the rumors say, who built them? Who else but the ancients! There was no one else who could have built them. And if the ancients are the makers, those towers are old and must have survived the destructions of the war. I want to see what kind of magic can make buildings survive what has shattered the earth."),
            dark_gray_book_line_bytes("You skip several pages containing a description of the voyage to the towers.", false),
            plain_book_line_bytes("I have forced my way into one of the towers. Magical they are, for sure, and guarded by the living dead. Fighting my way inside nearly exhausted me, and all I could do was grab some parchments and a small bag and flee, before those undead came back in greater numbers."),
            plain_book_line_bytes("The book is written in the language of the ancients. Unfortunately, I can barely understand some words. The bag contained polished pieces of bone, each bearing a rune. I will return to Exkordon now, and study them at my leisure."),
            plain_book_line_bytes("I found some pictures in the book, showing how to arrange the runes. I wonder what will happen..."),
            dark_gray_book_line_bytes("You notice a change in the writing. It is the same hand, but the letters are bigger, and more forcefully written.", false),
            plain_book_line_bytes("That does it. Scarewind is a weak fool. I shall kill him, and take Exkordons fate into my own hands."),
            plain_book_line_bytes("Easy, almost too easy it was. I am now Governor of Exkordon. Scarcewind died like the fool he was in life. 'How can you do that? Why? I trusted you!' What a fool. I invited him into my house, told him about an important discovery I made. He came, and left his guards outside. And so he died. When his guards came looking for him, I lured them into my cellars, and disposed of them. They are no match for the ancient's magic."),
            dark_gray_book_line_bytes("Here, the writing changes back to the style used in the beginning.", false),
            plain_book_line_bytes("What have I done? What came over me? And why are the dead rising, and walking my halls? They are dead! Dead! I killed them!"),
        ],
        BOOK_BONES1 => vec![
            plain_book_line_bytes("Day 12, year 45. Personal diary of Sluiran of the Caremar."),
            plain_book_line_bytes("The battles raging outside are closer to our hiding place. We must find some means to defend ourselves. I have started to study the forbidden art of necromancy, based on the rune magic. The undead shall fight where the living cannot."),
            dark_gray_book_line_bytes("You skip some pages.", false),
            plain_book_line_bytes("Day 37, year 47. Personal diary of Sluiran of the Caremar."),
            plain_book_line_bytes("The towers have fallen, but the undead have held our halls against the first wave of attackers. I have many, many bodies for my work now. More and more undead shall defend us. We might survive, after all."),
        ],
        BOOK_GWENDYLON => vec![dark_gray_book_line_bytes("Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles.", false)],
        BOOK_MADMAGES_BOOK1 => vec![
            plain_book_line_bytes("It is a long list of names, the masters and teachers of the mages order. With deep respect, the great past masters of the mages order, are here honoured."),
            plain_book_line_bytes("Wijn, the old one. Gree-Dli, master of summoning spells, Leerea, the empat, Djurna bridgecaller, Friize the recluse, Loisan creator. "),
            dark_gray_book_line_bytes("At the bottom of the following page you find a list of the current teachers of the mages order: ", true),
            dark_gray_book_line_bytes("It appears that someone has attempted to scratch away the final name from the parchment.", false),
        ],
        _ => book_text_lines(kind)
            .iter()
            .map(|line| plain_book_line_bytes(line))
            .collect(),
    }
}

pub(crate) fn demon_book_line_bytes(kind: u8, reader_id: u32) -> Vec<Vec<u8>> {
    let ritual = demon_ritual_words(reader_id, u32::from(kind - 13));
    let line = match kind {
        BOOK_DEMON1 => format!(
            "I have seen in written in fiery letters upon the sky: Those who have the knowledge can invoke protection against demonic might by uttering the words: '{ritual}'"
        ),
        BOOK_DEMON2 => format!(
            "Those who need better protection against earth demons, those who have the knowledge, use these words: '{ritual}'"
        ),
        BOOK_DEMON3..=BOOK_DEMON5 => format!("'{ritual}' will give thee even better protection."),
        _ => return Vec::new(),
    };
    vec![plain_book_line_bytes(&line)]
}

pub fn demon_ritual_words(reader_id: u32, nr: u32) -> String {
    const SYLLABLES: [&str; 10] = [
        "shir", "ka", "dor", "lagh", "kir", "dul", "arl", "sli", "dlu", "usga",
    ];
    const LEADS: [&str; 5] = ["ki", "do", "sa", "mi", "ru"];

    let mut val = id_rand(reader_id, nr);
    let v1 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 4;
    let v2 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 3;
    let v3 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 5;
    let v4 = (val % SYLLABLES.len() as u32) as usize;
    let lead = LEADS.get(nr as usize).copied().unwrap_or(LEADS[0]);

    format!(
        "{}{} {}{}{}",
        SYLLABLES[v1], SYLLABLES[v2], lead, SYLLABLES[v3], SYLLABLES[v4]
    )
}

pub(crate) fn id_rand(base: u32, step: u32) -> u32 {
    const VALUES: [u32; 16] = [
        0x12345678, 0x87654321, 0x17263524, 0xabef53ac, 0xbd341ace, 0x1045fe45, 0xea6deb2a,
        0x1d40fb4a, 0x1a83be1d, 0x1d441eff, 0x1a15e63f, 0x192502de, 0x90ae3ce2, 0x1de94be3,
        0x1e358f3b, 0xa1e3ff56,
    ];
    let mut ret = base
        .wrapping_add(step)
        .wrapping_add(base.wrapping_mul(step));
    for _ in 0..4 {
        ret ^= VALUES[(ret % VALUES.len() as u32) as usize];
    }
    ret
}

pub(crate) fn edemon_sign_line_bytes(demon_value: i32, readable_lines: &[&str]) -> Vec<Vec<u8>> {
    if demon_value < 1 {
        return vec![plain_book_line_bytes(
            "It's written in strange letters you cannot read.",
        )];
    }
    if demon_value < 2 {
        return vec![plain_book_line_bytes(
            "You recognice some of the letters used in this sign from your studies of the ancient knowledge, but you cannot tell what the sign means.",
        )];
    }
    readable_lines
        .iter()
        .map(|line| plain_book_line_bytes(line))
        .collect()
}

pub fn book_nook_joke_line_bytes(roll: u32) -> Vec<Vec<u8>> {
    let lines: &[&str] = match roll % 5 {
        0 => &[
            "What did the fisherman say to the card magician?",
            "Pick a cod, any cod!",
        ],
        1 => &[
            "Who can shave 25 times a day and still have a beard?",
            "A barber.",
        ],
        2 => &[
            "Did you hear about the fire at the circus?",
            "It was in tents.",
        ],
        3 => &[
            "What did the rude prism say to the light beam that smacked into him?",
            "Get bent!",
        ],
        _ => &["What bone will a dog never eat?", "A trombone."],
    };
    lines
        .iter()
        .map(|line| plain_book_line_bytes(line))
        .collect()
}

pub fn book_special_effect(kind: u8) -> Option<u32> {
    match kind {
        BOOK_EDEMON3 => Some(50287),
        BOOK_EDEMON4 => Some(50305),
        _ => None,
    }
}

pub(crate) fn plain_book_line_bytes(line: &str) -> Vec<u8> {
    line.as_bytes().to_vec()
}

pub(crate) fn dark_gray_book_line_bytes(
    line: &str,
    reset_before_current_teachers: bool,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len() + 32);
    out.extend_from_slice(COL_DARK_GRAY);
    out.extend_from_slice(line.as_bytes());
    if reset_before_current_teachers {
        out.extend_from_slice(COL_RESET);
        out.extend_from_slice(b"Bretl, Anna-Sofia, Leaner, Crem, Guiwynn.");
    }
    out
}

pub(crate) fn book_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::BookText {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
        demon_value: i32::from(character.values[1][CharacterValue::Demon as usize]),
    }
}
