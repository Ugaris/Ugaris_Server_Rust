//! Legacy `CDT_*`/`CDR_*` dispatch-type and driver ids plus `NT_*`/`NTID_*`
//! notify constants (`src/system/drvlib.h`, `src/include/notify.h`).

pub const CDT_DRIVER: u16 = 0;
pub const CDT_ITEM: u16 = 1;
pub const CDT_DEAD: u16 = 2;
pub const CDT_RESPAWN: u16 = 3;
pub const CDT_SPECIAL: u16 = 4;
/// C `#define CDR_ACLERK 4` (`src/system/drvlib.h`): the arena clerk in
/// Cameron (`src/module/merchants/merchant.c::aclerk_driver`).
pub const CDR_ACLERK: u16 = 4;
pub const CDR_LOSTCON: u16 = 5;
pub const CDR_MERCHANT: u16 = 6;
pub const CDR_SIMPLEBADDY: u16 = 7;
/// C `#define CDR_BANK 22` (`src/system/drvlib.h`): generic bank driver.
pub const CDR_BANK: u16 = 22;
pub const CDR_MACRO: u16 = 37;
pub const CDR_SWAMPCLARA: u16 = 54;
/// C `#define CDR_PROFESSOR 55` (`src/system/drvlib.h`): the generic
/// profession-teacher NPC (`src/common/professor.c`), see
/// `world::npc::professor`'s module doc comment.
pub const CDR_PROFESSOR: u16 = 55;
pub const CDR_SWAMPMONSTER: u16 = 56;
pub const CDR_PALACEISLENA: u16 = 57;
/// C `#define CDR_FORESTIMP 58` (`src/system/drvlib.h`): the treasure-
/// hinting imp (`src/area/16/forest.c::imp_driver`), see
/// `world::npc::area16::imp`'s module doc comment.
pub const CDR_FORESTIMP: u16 = 58;
/// C `#define CDR_FORESTMONSTER 59` (`src/system/drvlib.h`): area 16's
/// wolves/bears/skeletons. C's own `ch_driver` dispatch
/// (`forest.c:909-911`) is an unconditional tail call to
/// `char_driver(CDR_SIMPLEBADDY, ...)`, so `CDR_FORESTMONSTER` characters
/// reuse the SimpleBaddy AI end-to-end - same precedent as `CDR_PENTER`/
/// `CDR_SWAMPMONSTER` (see the `character.driver == CDR_SIMPLEBADDY`
/// gates widened alongside those in `world/npc_fight.rs`/
/// `world/npc_idle.rs`). Its own `ch_died_driver`/`monster_dead` death
/// hook (`forest.c:817-853`) lives in `World::
/// apply_forest_monster_death_driver` (item weapon-glow half) plus
/// `ugaris-server`'s `apply_forest_monster_death_from_hurt_event`
/// (`imp_kills`/`hermit_state` halves, which need `PlayerRuntime`).
pub const CDR_FORESTMONSTER: u16 = 59;
/// C `#define CDR_FORESTWILLIAM 60` (`src/system/drvlib.h`): the
/// bear-hunt/mantis-stew quest giver (`src/area/16/forest.c::
/// william_driver`), see `world::npc::area16::william`'s module doc
/// comment.
pub const CDR_FORESTWILLIAM: u16 = 60;
/// C `#define CDR_FORESTHERMIT 61` (`src/system/drvlib.h`): the
/// spider-queen quest giver (`src/area/16/forest.c::hermit_driver`), see
/// `world::npc::area16::hermit`'s module doc comment.
pub const CDR_FORESTHERMIT: u16 = 61;
/// C `#define CDR_NOMAD 73` (`src/system/drvlib.h:129`, comment "nomad:
/// nomad"): the Nomad Plains tribe NPCs - Kalanur (tribe recruiter, quest
/// 32), Irakar (dice seller), the game host, the two Kir monastery monks
/// (Sarkilar quest 33, life-teacher quest 34), and the statue seller -
/// all six personas share this one driver, differentiated at spawn time
/// by their own `arg="nr=N;..."` (`src/area/19/nomad.c::nomad`), see
/// `world::npc::area19::nomad`'s module doc comment.
pub const CDR_NOMAD: u16 = 73;
/// C `#define CDR_MADHERMIT 76` (`src/system/drvlib.h:132`, comment
/// "nomad: mad hermit"): the flower-guarding hermit in the Nomad Plains
/// (`src/area/19/nomad.c::madhermit_driver`), see `world::npc::
/// area19::madhermit`'s module doc comment.
pub const CDR_MADHERMIT: u16 = 76;
/// C `#define CDR_TWOGUARD 62` (`src/system/drvlib.h`): the Exkordon city
/// guard patrol (`src/area/17/two.c::guard_driver`), see `world::npc::
/// area17::guard`'s module doc comment.
pub const CDR_TWOGUARD: u16 = 62;
/// C `#define CDR_TWOBARKEEPER 63` (`src/system/drvlib.h`): the Two-Towns
/// tavern barkeeper, "guest pass" broker (`src/area/17/two.c::barkeeper`),
/// see `world::npc::area17::barkeeper`'s module doc comment.
pub const CDR_TWOBARKEEPER: u16 = 63;
/// C `#define CDR_TWOSERVANT 65` (`src/system/drvlib.h`, comment "servant
/// in forbidden territory"): the palace maids/mistress/governor's-double
/// NPCs (`src/area/17/two.c::servant`), see `world::npc::area17::servant`'s
/// module doc comment.
pub const CDR_TWOSERVANT: u16 = 65;
/// C `#define CDR_TWOTHIEFGUARD 66` (`src/system/drvlib.h`, comment
/// "thieves guild guard"): the entrance guard to the Exkordon thieves
/// guild sewers (`src/area/17/two.c::thiefguard`), see `world::npc::
/// area17::thiefguard`'s module doc comment.
pub const CDR_TWOTHIEFGUARD: u16 = 66;
/// C `#define CDR_TWOTHIEFMASTER 67` (`src/system/drvlib.h`, comment
/// "two cities: thieves guild master"): the lockpick-chain quest giver
/// behind the sewer entrance (`src/area/17/two.c::thiefmaster`), see
/// `world::npc::area17::thiefmaster`'s module doc comment.
pub const CDR_TWOTHIEFMASTER: u16 = 67;
/// C `#define CDR_TWOROBBER 68` (`src/system/drvlib.h`, comment "robber
/// (simple baddy with special death)"): the Exkordon forest-camp robbers
/// (`robber1`-`robber4`/`robber_guard`/`robber_baron` templates,
/// `zones/17/two.chr`). C's own `ch_driver` dispatch (`two.c:3163-3165`)
/// is an unconditional tail call to `char_driver(CDR_SIMPLEBADDY, ...)`,
/// so `CDR_TWOROBBER` characters reuse the SimpleBaddy AI end-to-end -
/// same precedent as `CDR_PENTER`/`CDR_FORESTMONSTER` (see the
/// `character.driver == CDR_SIMPLEBADDY` gates widened alongside those in
/// `world/npc_fight.rs`/`world/npc_idle.rs`). Its own `ch_died_driver`/
/// `robber_dead` death hook (`two.c:2211-2247`) lives in `ugaris-server`'s
/// `apply_two_robber_death_from_hurt_event` (needs `PlayerRuntime`'s
/// `twocity_thief_state`/`twocity_thief_killed`).
pub const CDR_TWOROBBER: u16 = 68;
/// C `#define CDR_TWOSANWYN 69` (`src/system/drvlib.h`): the military
/// quest giver "Sanwyn" (`src/area/17/two.c::sanwyn`), see `world::npc::
/// area17::sanwyn`'s module doc comment.
pub const CDR_TWOSANWYN: u16 = 69;
pub const CDR_TWOSKELLY: u16 = 70;
/// C `#define CDR_TWOALCHEMIST 71` (`src/system/drvlib.h`): the
/// spider-poison quest giver "Cervik" (`src/area/17/two.c::alchemist`),
/// see `world::npc::area17::alchemist`'s module doc comment.
pub const CDR_TWOALCHEMIST: u16 = 71;
pub const CDR_TRADER: u16 = 72;
/// C `#define CDR_PENTER 64` (`src/system/drvlib.h`): pentagram-quest
/// guardian demons (`src/area/4/pents.c::demon_character_driver`). Its own
/// tail call is `char_driver(CDR_SIMPLEBADDY, ...)`, so `CDR_PENTER`
/// characters reuse the SimpleBaddy AI end-to-end - see the
/// `character.driver == CDR_SIMPLEBADDY` gates widened alongside
/// `CDR_DUNGEONFIGHTER` in `world/npc_fight.rs`/`world/npc_idle.rs`.
pub const CDR_PENTER: u16 = 64;
/// C `#define CDR_TESTER 77` (`src/system/drvlib.h`): the pentagram-quest
/// QA test bot (`src/area/4/pents.c::pentagram_tester_driver`) - see
/// `world::npc::area4::tester`'s module doc comment for why it is not
/// player-facing.
pub const CDR_TESTER: u16 = 77;
/// C `#define CDR_FDEMON_DEMON 46` (`src/system/drvlib.h`): the roaming
/// Fire Demon/Fire Golem monsters (`src/area/8/fdemon.c::fdemon_demon`),
/// see `world::npc::area8::fdemon_demon`'s module doc comment. Its own
/// `sprite==190` (the "Fire Golem" boss variant) branch tail-calls
/// `char_driver(CDR_SIMPLEBADDY, ...)` unconditionally every tick, so
/// those specific spawns are assigned `CDR_SIMPLEBADDY` directly at spawn
/// time instead (see `zone.rs`'s `CDR_FDEMON_DEMON` branch) - only the
/// non-190-sprite "Fire Demon" trash mobs actually run under this id.
pub const CDR_FDEMON_DEMON: u16 = 46;
/// C `#define CDR_PALACEGUARD 47` (`src/system/drvlib.h`): the palace
/// patrol/reserve-ambush demon sentries (`src/area/11/palace.c::
/// palace_guard`), see `world::npc::area11::palace_guard`'s module doc
/// comment.
pub const CDR_PALACEGUARD: u16 = 47;
/// C `#define CDR_FDEMON_BOSS 45` (`src/system/drvlib.h`): the underground
/// army Commander mission-giver (`src/area/8/fdemon.c::fdemon_boss`), see
/// `world::npc::area8::fdemon_boss`'s module doc comment.
pub const CDR_FDEMON_BOSS: u16 = 45;
/// C `#define CDR_FDEMON_ARMY 44` (`src/system/drvlib.h`): the
/// recruitable-soldier companion driver (`src/area/8/fdemon.c::
/// fdemon_army`), see `world::npc::area8::fdemon_army`'s module doc
/// comment.
pub const CDR_FDEMON_ARMY: u16 = 44;
pub const CDR_LQNPC: u16 = 74;
pub const CDR_JANITOR: u16 = 85;
/// C `#define CDR_GOLEMKEYHOLDER 107` (`src/system/drvlib.h:155`, comment
/// "mines: key to second mine area"): the boss golem spawned into a
/// private mine-vault room by `keyholder_door`/`IDR_MINEKEYDOOR`
/// (`src/area/12/mine.c::keyholder_door`), see
/// `world::npc::area12::golemkeyholder`'s module doc comment.
pub const CDR_GOLEMKEYHOLDER: u16 = 107;
pub const CDR_TEUFELDEMON: u16 = 114;
pub const CDR_TEUFELGAMBLER: u16 = 115;
pub const CDR_TEUFELQUEST: u16 = 116;
pub const CDR_TEUFELRAT: u16 = 117;
/// C `#define CDR_CALIGARGUARD 118` (`src/system/drvlib.h:166`): the two
/// entrance guards (Eulc/Margana) whose alternating "Human entry is not
/// permitted!"/"He let the bed in!" banter walks a player through the
/// "backwards is the key to entry" riddle
/// (`src/area/36/caligar.c::guard_driver`), see
/// `world::npc::area36::caligar_guard`'s module doc comment.
pub const CDR_CALIGARGUARD: u16 = 118;
/// C `#define CDR_CALIGARGLORI 119` (`src/system/drvlib.h:167`): Glori,
/// "First in charge" of the library, who runs the quest-54-58 obelisk/
/// key-part chain (`src/area/36/caligar.c::glori_driver`), see
/// `world::npc::area36::glori`'s module doc comment.
pub const CDR_CALIGARGLORI: u16 = 119;
/// C `#define CDR_CALIGARARQUIN 120` (`src/system/drvlib.h:168`): Arquin,
/// stationed outside the library, who explains the obelisks/dungeon key
/// and points the player at Homden (`src/area/36/caligar.c::
/// arquin_driver`), see `world::npc::area36::arquin`'s module doc comment.
pub const CDR_CALIGARARQUIN: u16 = 120;
/// C `#define CDR_CALIGARSMITH 121` (`src/system/drvlib.h:169`): the dwarf
/// blacksmith who forges the three key parts into the underground key for
/// 5,000 gold and later sells a translation dictionary for 10,000 gold
/// (`src/area/36/caligar.c::smith_driver`), see
/// `world::npc::area36::smith`'s module doc comment.
pub const CDR_CALIGARSMITH: u16 = 121;
/// C `#define CDR_CALIGARHOMDEN 122` (`src/system/drvlib.h:170`): Homden,
/// the banished Carmin Clan brother who opens quest 59 (find his stolen
/// ring) and narrates the palace/Emperor backstory
/// (`src/area/36/caligar.c::homden_driver`), see
/// `world::npc::area36::homden`'s module doc comment.
pub const CDR_CALIGARHOMDEN: u16 = 122;
/// C `#define CDR_CALIGARGUARD2 123` (`src/system/drvlib.h:171`): a
/// combat-capable Caligar guard that taunts ("Halt! You will die where
/// you stand!") before falling through to the plain `CDR_SIMPLEBADDY`
/// self-defense/idle AI (`src/area/36/caligar.c::guard2_driver`), see
/// `world::npc::area36::caligar_guard2`'s module doc comment.
pub const CDR_CALIGARGUARD2: u16 = 123;
pub const CDR_CALIGARSKELLY: u16 = 124;
/// C `#define CDR_NOP 136` (`src/system/drvlib.h:186`, comment "arkhata"):
/// the Fighting School's stationary background "Student" NPCs
/// (`src/area/37/arkhata.c::nop_driver`), see `world::npc::area37::nop`'s
/// module doc comment.
pub const CDR_NOP: u16 = 136;
/// C `#define CDR_BOOKEATER 140` (`src/system/drvlib.h:190`, comment
/// "arkhata"): "The Book Eater" monster, quest 70's target. C's own
/// `ch_driver` dispatch (`arkhata.c:4583-4585`) is an unconditional tail
/// call to `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`
/// (`bookeater_driver`, `arkhata.c:2083-2085`), reusing the SimpleBaddy
/// driver's full idle-wander/auto-attack AI wholesale - same precedent as
/// `CDR_ARKHATAPRISON` below (`bookeater_dead`'s own quest-70 completion
/// check, ported separately as `ugaris-server::world_events::
/// death_hooks::apply_arkhata_bookeater_death_from_hurt_event`, is the
/// only other C-visible behavior this driver has).
pub const CDR_BOOKEATER: u16 = 140;
/// C `#define CDR_ARKHATASKELLY 138` (`src/system/drvlib.h:188`, comment
/// "arkhata"): the Fighting School's respawning skeleton monsters
/// (`Skeleton_for_final_area`, `zones/37/Vamp_Skele_Zombie.chr`). C's own
/// `ch_driver` dispatch (`arkhata.c:4620-4622`) is an unconditional tail
/// call to `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`
/// (`arkhataskelly_driver`, `arkhata.c:1587-1609`), reusing the
/// SimpleBaddy driver's full idle-wander/auto-attack AI wholesale - same
/// precedent as `CDR_BOOKEATER` above. The driver's only other behavior
/// is a purely internal idle-tick position-hash bookkeeping array used
/// solely to count still-alive arkhataskellies inside its own
/// `arkhataskelly_dead` (`:1612-1646`, ported as `ugaris-server::
/// world_events::death_hooks::
/// apply_arkhataskelly_death_from_hurt_event`, which counts living
/// `CDR_ARKHATASKELLY` characters directly instead - behaviorally
/// equivalent, not observable to players).
pub const CDR_ARKHATASKELLY: u16 = 138;
/// C `#define CDR_ARKHATAPRISON 151` (`src/system/drvlib.h:200`, comment
/// "arkhata"): the Fortress prisoner NPC. C's own `ch_driver` dispatch
/// (`arkhata.c:4616-4618`) is an unconditional tail call to
/// `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`
/// (`prisoner_driver`, `arkhata.c:4329-4331`), reusing the SimpleBaddy
/// driver's full idle-wander/auto-attack AI wholesale - same precedent as
/// `CDR_TEUFELRAT`/`CDR_CALIGARSKELLY` above (`prisoner_dead`'s own
/// "I know the secret, it's right here!" line, ported separately as
/// `ugaris-server::world_events::death_hooks::
/// apply_arkhata_prisoner_death_from_hurt_event`, is the only other
/// C-visible behavior this driver has).
pub const CDR_ARKHATAPRISON: u16 = 151;
/// C `#define CDR_FORTRESSGUARD 143` (`src/system/drvlib.h:193`, comment
/// "arkhata"): the Arkhata Fortress guards. Unlike the plain SimpleBaddy
/// tail calls above, `fortressguard_driver` (`arkhata.c:2587-2833`) is its
/// own reimplementation of `simple_baddy_driver` (`src/module/
/// simple_baddy.c:159-422`) over the same `DRD_SIMPLEBADDYDRIVER` storage
/// slot (own locally-renamed `struct fortressguard_driver_data`, byte-
/// identical field list) with exactly two behavioral deltas: (1) its own
/// `NT_CHAR` case inlines the `aggressive && is_valid_enemy(...)` check
/// itself, adding an extra `!has_item(co, IID_ARKHATA_LETTER5)` guard -
/// entrance-pass holders are never aggroed on sighting - then always
/// calls the trailing `standard_message_driver(cn, msg, 0, dat->helper)`
/// with `aggressive` forced to `0` to avoid double-adding (this repo's
/// [`SimpleBaddyMessageOutcome::StandardAggro`] `hurtme: false` variant,
/// filtered by driver id in `world::npc_messages`); (2) it has no
/// `NT_GOTHIT` potion-drinking case at all, which is a no-op difference
/// since `fortressguard_driver_data` has no `drinkInvPots` field to begin
/// with (`fortressguard_driver_parse` never accepts a `drinkinvpots=`
/// arg), so C's own `dat->drinkInvPots` gate there would always have been
/// false anyway. `NT_GOTHIT` self-defense (`fight_driver_add_enemy`
/// via `standard_message_driver`, `hurtme: true`) is untouched and still
/// fires for entrance-pass holders exactly like any other SimpleBaddy -
/// only the initial-sighting aggro is suppressed. `ch_died_driver`/
/// `ch_respawn_driver` (`arkhata.c:4675-4676,4738-4739`) are both bare
/// `return 1` (no death hook, standard respawn), same as `CDR_BRIDGEGUARD`.
pub const CDR_FORTRESSGUARD: u16 = 143;
/// C `#define CDR_RAMMY 131` (`src/system/drvlib.h:181`, comment
/// "arkhata"): the ruler of Arkhata, quest 65 ("Rammy's Crown") and quest
/// 71 ("Entrance Passes") giver, see `world::npc::area37::rammy`'s module
/// doc comment.
pub const CDR_RAMMY: u16 = 131;
/// C `#define CDR_JAZ 132` (`src/system/drvlib.h:182`, comment "arkhata"):
/// the Arkhata townsman who runs "Ishtar's Bracelet" (quest 66), see
/// `world::npc::area37::jaz`'s module doc comment.
pub const CDR_JAZ: u16 = 132;
/// C `#define CDR_BRIDGEGUARD 133` (`src/system/drvlib.h:183`, comment
/// "arkhata"): the bridge-crossing guards outside Arkhata proper, see
/// `world::npc::area37::bridgeguard`'s module doc comment.
pub const CDR_BRIDGEGUARD: u16 = 133;
/// C `#define CDR_FIONA 134` (`src/system/drvlib.h:184`, comment
/// "arkhata"): the Fighting School headmistress, quest 67 ("The Missing
/// Ring") giver and student-challenge/skill-raise NPC, see
/// `world::npc::area37::fiona`'s module doc comment.
pub const CDR_FIONA: u16 = 134;
/// C `#define CDR_GLADIATOR 135` (`src/system/drvlib.h:185`, comment
/// "arkhata"): the disposable student opponent `fight_student`
/// (`world::npc::area37::fiona`) spawns for Fiona's "enter" challenge, see
/// `world::npc::area37::gladiator`'s module doc comment.
pub const CDR_GLADIATOR: u16 = 135;
/// C `#define CDR_RAMIN 137` (`src/system/drvlib.h:187`, comment
/// "arkhata"): the Arkhata civil officer who runs "A Shopkeeper's Fright"
/// (quest 68), see `world::npc::area37::ramin`'s module doc comment.
pub const CDR_RAMIN: u16 = 137;
/// C `#define CDR_ARKHATAMONK 139` (`src/system/drvlib.h:189`, comment
/// "arkhata"): the four monk personas (Gregor/Johan/Johnatan/Tracy)
/// sharing one dialogue state machine, quests 69/70/78, see
/// `world::npc::area37::arkhatamonk`'s module doc comment.
pub const CDR_ARKHATAMONK: u16 = 139;
/// C `#define CDR_CAPTAIN 141` (`src/system/drvlib.h:191`, comment
/// "arkhata"): the Fortress Captain, first stop of the entrance-pass-
/// system chain, see `world::npc::area37::captain`'s module doc comment.
pub const CDR_CAPTAIN: u16 = 141;
/// C `#define CDR_JUDGE 142` (`src/system/drvlib.h:192`, comment
/// "arkhata"): the fortress judge who writes the entrance-pass letters,
/// see `world::npc::area37::judge`'s module doc comment.
pub const CDR_JUDGE: u16 = 142;
/// C `#define CDR_JADA 144` (`src/system/drvlib.h:194`, comment
/// "arkhata"): the Arkhata mystic who runs "The Source" (quest 72), see
/// `world::npc::area37::jada`'s module doc comment.
pub const CDR_JADA: u16 = 144;
/// C `#define CDR_POTMAKER 145` (`src/system/drvlib.h:195`, comment
/// "arkhata"): the Arkhata craftsman who runs "A Special Pot" (quest 73),
/// see `world::npc::area37::potmaker`'s module doc comment.
pub const CDR_POTMAKER: u16 = 145;
/// C `#define CDR_HUNTER 146` (`src/system/drvlib.h:196`, comment
/// "arkhata"): the Arkhata hunter who runs "The Blue Harpy" (quest 77),
/// see `world::npc::area37::hunter`'s module doc comment.
pub const CDR_HUNTER: u16 = 146;
/// C `#define CDR_THAIPAN 147` (`src/system/drvlib.h:197`, comment
/// "arkhata"): the Arkhata monk who runs "The Ancient Scroll" (quest 74)
/// and the repeatable Buddah Statue hand-in, see `world::npc::area37::
/// thaipan`'s module doc comment.
pub const CDR_THAIPAN: u16 = 147;
/// C `#define CDR_TRAINER 148` (`src/system/drvlib.h:198`, comment
/// "arkhata"): the Fighting School combat trainer who runs "A Kidnapped
/// Student" (quest 75), see `world::npc::area37::trainer`'s module doc
/// comment.
pub const CDR_TRAINER: u16 = 148;
/// C `#define CDR_KIDNAPPEE 149` (`src/system/drvlib.h:199`, comment
/// "arkhata"): the trainer's kidnapped student, rescued as part of quest
/// 75, see `world::npc::area37::kidnappee`'s module doc comment.
pub const CDR_KIDNAPPEE: u16 = 149;
/// C `#define CDR_ARKHATACLERK 150` (`src/system/drvlib.h:200`, comment
/// "arkhata"): the Fortress clerk who runs "The Traitors" (quest 76), see
/// `world::npc::area37::clerk`'s module doc comment.
pub const CDR_ARKHATACLERK: u16 = 150;
/// C `#define CDR_KRENACH 152` (`src/system/drvlib.h:202`, comment
/// "arkhata"): the dwarf grandfather who closes out quest 78 ("The
/// Mysterious Language") and refunds part of the Monk Dictionary's cost,
/// see `world::npc::area37::krenach`'s module doc comment.
pub const CDR_KRENACH: u16 = 152;
/// C `#define CDR_LAB2HERALD 196` (`src/system/drvlib.h:222`): the lab2
/// graveyard chapel keeper (`src/area/22/lab2.c::lab2_herald_driver`), see
/// `world::npc::area22::lab2_herald`'s module doc comment.
pub const CDR_LAB2HERALD: u16 = 196;
/// C `#define CDR_LAB2DEAMON 197` (`src/system/drvlib.h:223`): the family-
/// vault masquerade-detection guardian (`src/area/22/lab2.c::
/// lab2_deamon_driver`), see `world::npc::area22::lab2_deamon`'s module
/// doc comment.
pub const CDR_LAB2DEAMON: u16 = 197;
pub const CDR_LAB2UNDEAD: u16 = 198;
/// C `#define CDR_LABGNOMEDRIVER 199` (`src/system/drvlib.h:225`): the
/// area-22 Lab 1 torch-gnome triad (guard/fighter/immortal master).
pub const CDR_LABGNOMEDRIVER: u16 = 199;
/// C `#define CDR_LAB3PASSGUARD 194` (`src/system/drvlib.h:220`): the
/// lab3 password-gate guard (`src/area/22/lab3.c::lab3_passguard_driver`),
/// see `world::npc::area22::lab3_passguard`'s module doc comment.
pub const CDR_LAB3PASSGUARD: u16 = 194;
/// C `#define CDR_LAB3PRISONER 195` (`src/system/drvlib.h:221`): the
/// lab3 mute prisoner note-giver (`src/area/22/lab3.c::
/// lab3_prisoner_driver`), see `world::npc::area22::lab3_prisoner`'s
/// module doc comment.
pub const CDR_LAB3PRISONER: u16 = 195;
/// C `#define CDR_LAB4SEYAN 192` (`src/system/drvlib.h:218`): the lab4
/// seyan quest giver (`src/area/22/lab4.c::lab4_seyan_driver`), see
/// `world::npc::area22::lab4_seyan`'s module doc comment.
pub const CDR_LAB4SEYAN: u16 = 192;
/// C `#define CDR_LAB4GNALB 193` (`src/system/drvlib.h:219`): the lab4
/// gnalb guard/crazy-gnalb driver (`src/area/22/lab4.c::
/// lab4_gnalb_driver`), see `world::npc::area22::lab4_gnalb`'s module
/// doc comment.
pub const CDR_LAB4GNALB: u16 = 193;
/// C `#define CDR_LAB5DAEMON 189` (`src/system/drvlib.h:215`): the lab5
/// master/servant/gunned demon fight driver (`src/area/22/lab5.c::
/// lab5_daemon_driver`), see `world::npc::area22::lab5_daemon`'s module
/// doc comment.
pub const CDR_LAB5DAEMON: u16 = 189;
/// C `#define CDR_LAB5MAGE 190` (`src/system/drvlib.h:216`): the lab5
/// mage "Mathor" (`src/area/22/lab5.c::lab5_mage_driver`), see
/// `world::npc::area22::lab5_mage`'s module doc comment.
pub const CDR_LAB5MAGE: u16 = 190;
/// C `#define CDR_LAB5SEYAN 191` (`src/system/drvlib.h:217`): the lab5
/// "Laros" quest giver who collects the three demon heads
/// (`src/area/22/lab5.c::lab5_seyan_driver`), see `world::npc::area22::
/// lab5_seyan`'s module doc comment.
pub const CDR_LAB5SEYAN: u16 = 191;
/// C `#define CDR_STRATEGY_BOSS 80` (`src/system/drvlib.h:129`): Cinciac,
/// the Ice Army Caves commander mission-giver (`src/area/23_24/
/// strategy.c::strategy_boss`), see `world::npc::area23_24::boss`'s
/// module doc comment. The command-table `CDR_STRATEGY_PARSER = 79`
/// (already ported as `World::apply_strategy_special_command`, dispatched
/// directly from `tick_client_actions.rs` without needing a driver id
/// constant of its own) has no Rust constant either.
pub const CDR_STRATEGY_BOSS: u16 = 80;
/// C `#define CDR_STRATEGY 78` (`src/system/drvlib.h:126`): the
/// recruitable worker/fighter/miner NPC (`src/area/23_24/
/// strategy.c::strategy_driver`), see `world::npc::area23_24::worker`'s
/// module doc comment. No live character can run on this driver yet -
/// `spawner_sub`/`take_spawner` spawning is still unported - so it's only
/// reachable via directly-constructed test characters for now.
pub const CDR_STRATEGY: u16 = 78;
/// C `#define CDR_CAMHERMIT 14` (`src/system/drvlib.h`): the forest
/// hermit NPC in area 1 (`src/area/1/gwendylon.c::camhermit_driver`).
pub const CDR_CAMHERMIT: u16 = 14;
/// C `#define CDR_YOAKIN 9` (`src/system/drvlib.h`): the area-1 hunter
/// quest giver at the knight castle (`src/area/1/gwendylon.c::
/// yoakin_driver`).
pub const CDR_YOAKIN: u16 = 9;
/// C `#define CDR_TERION 11` (`src/system/drvlib.h`): the ambient lore NPC
/// in area 1's village (`src/area/1/gwendylon.c::terion_driver`).
pub const CDR_TERION: u16 = 11;
/// C `#define CDR_GWENDYLON 8` (`src/system/drvlib.h`): the area-1 main
/// quest-giver mage at the knight castle
/// (`src/area/1/gwendylon.c::gwendylon_driver`).
pub const CDR_GWENDYLON: u16 = 8;
/// C `#define CDR_GREETER 13` (`src/system/drvlib.h`): the "specific NPC
/// in area1, stronghold" (Cameron, the tutorial-town Governor) greeting
/// NPC (`src/area/1/gwendylon.c::greeter_driver`).
pub const CDR_GREETER: u16 = 13;
/// C `#define CDR_JESSICA 125` (`src/system/drvlib.h`, "Cameron: robbers"):
/// the area-1 robber-operations quest NPC
/// (`src/area/1/gwendylon.c::jessica_driver`).
pub const CDR_JESSICA: u16 = 125;
/// C `#define CDR_JIU 127` (`src/system/drvlib.h`, "Cameron: Jiu"): the
/// riverbeast quest-giving pilgrim NPC
/// (`src/area/1/gwendylon.c::jiu_driver`).
pub const CDR_JIU: u16 = 127;
/// C `#define CDR_RIVERBEAST 128` (`src/system/drvlib.h`, "Cameron:
/// Riverbeast (Jiu Quest)"): the killable beast whose death
/// (`src/area/1/gwendylon.c::riverbeast_dead`, `:2255-2272`) advances
/// `CDR_JIU`'s quest chain from `JIU_STATE_WAIT_FOR_KILL` to
/// `JIU_STATE_BEAST_KILLED`. See
/// `crate::world::jiu`/`ugaris-server::world_events::
/// apply_riverbeast_death_from_hurt_event`.
pub const CDR_RIVERBEAST: u16 = 128;
/// C `#define CDR_CAMERON_FORESTMONSTER 129` (`src/system/drvlib.h`,
/// "Cameron: Mobs for stone circle"): area 1's forest bear monster, whose
/// death (`src/area/1/gwendylon.c::monster_dead`, `:5201-5231`) increments
/// `CDR_CAMHERMIT`'s `camhermit_kills` counter and separately re-glows a
/// worn weapon at noon in the stone-circle area. See
/// `ugaris-server::world_events::apply_area1_monster_death_from_hurt_event`
/// and `World::apply_area1_monster_death_driver`.
pub const CDR_CAMERON_FORESTMONSTER: u16 = 129;
/// C `#define CDR_BREDEL 154` (`src/system/drvlib.h`, "Cameron: Bredel
/// driver"): the robber-operations boss whose death
/// (`src/area/1/gwendylon.c::bredel_dead`, `:2825-2842`) advances
/// `CDR_JESSICA`'s quest chain from `JESSICA_STATE_QUEST2_DO` to
/// `JESSICA_STATE_QUEST2_FINISH`. See
/// `crate::world::jessica`/`ugaris-server::world_events::
/// apply_bredel_death_from_hurt_event`.
pub const CDR_BREDEL: u16 = 154;
/// C `#define CDR_FOREST_RANGER 155` (`src/system/drvlib.h`, "Cameron:
/// Forest Ranger (warns from bigbadspider)"): the stationary
/// bear-attack-warning sentry near area 1's stone circle
/// (`src/area/1/gwendylon.c::forest_ranger_driver`).
pub const CDR_FOREST_RANGER: u16 = 155;
/// C `#define CDR_BRITHILDIE 126` (`src/system/drvlib.h`, "Cameron:
/// Brithildie"): the Governor's-mother ambient lore NPC
/// (`src/area/1/gwendylon.c::brithildie_driver`) who unlocks
/// `QLOG_BRITHILDIE`.
pub const CDR_BRITHILDIE: u16 = 126;
/// C `#define CDR_BIGBADSPIDER 153` (`src/system/drvlib.h`, "Cameron:
/// BigBadSpider driver"): the killable spider whose death
/// (`src/area/1/gwendylon.c::bigbadspider_dead`, `:2850-2870`) completes
/// `CDR_BRITHILDIE`'s `QLOG_BRITHILDIE` quest, advancing
/// `BRITHILDIE_STATE_NOMORETALES_QOPEN` to `_QDONE`. Dispatched through
/// `CDR_SIMPLEBADDY` for its own combat driver (`gwendylon.c:6138`), same
/// as `CDR_RIVERBEAST`/`CDR_BREDEL`/`CDR_CAMERON_FORESTMONSTER` above. See
/// `ugaris-server::world_events::apply_bigbadspider_death_from_hurt_event`.
pub const CDR_BIGBADSPIDER: u16 = 153;
/// C `#define CDR_JAMES 12` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - the town drunkard's Lydia-quest hand-off/
/// hardcore-recruiter/paid-advice NPC (`src/area/1/gwendylon.c::
/// james_driver`).
pub const CDR_JAMES: u16 = 12;
/// C `#define CDR_NOOK 15` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - the identity-crisis jester/judge/knight NPC
/// (`src/area/1/gwendylon.c::nook_driver`).
pub const CDR_NOOK: u16 = 15;
/// C `#define CDR_LYDIA 16` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - Gwendylon's hungover daughter and her
/// hangover-potion quest chain (`src/area/1/gwendylon.c::lydia_driver`).
pub const CDR_LYDIA: u16 = 16;
/// C `#define CDR_GATE_WELCOME 39` (`src/system/drvlib.h`): the stationary
/// gatekeeper-welcome NPC (`gate_welcome` template,
/// `src/system/gatekeeper.c::gate_welcome_driver`).
pub const CDR_GATE_WELCOME: u16 = 39;
/// C `#define CDR_ROBBER 17` (`src/system/drvlib.h`, "specific enemy in
/// area1, wood"): the midnight-meeting forest patrol NPC
/// (`src/area/1/gwendylon.c::robber_driver`).
pub const CDR_ROBBER: u16 = 17;
/// C `#define CDR_SANOA 18` (`src/system/drvlib.h`, "specific NPC (enemy)
/// in area1, city"): the ambient twelve-waypoint city walker
/// (`src/area/1/gwendylon.c::sanoa_driver`), no dialogue at all.
pub const CDR_SANOA: u16 = 18;
/// C `#define CDR_BALLTRAP 10` (`src/system/drvlib.h`, "specific enemy,
/// area1, wood"): the stationary ball-trap-mechanism guard skeleton
/// (`src/area/1/gwendylon.c::balltrap_skelly_driver`).
pub const CDR_BALLTRAP: u16 = 10;
/// C `#define CDR_ASTURIN 19` (`src/system/drvlib.h`, "specific NPC
/// (enemy) in area1, city"): the private-quarters guard NPC
/// (`src/area/1/gwendylon.c::asturin_driver`).
pub const CDR_ASTURIN: u16 = 19;
/// C `#define CDR_RESKIN 20` (`src/system/drvlib.h`, "specific NPC in
/// area1, city"): the tavern-keeper/alchemy-turn-in NPC
/// (`src/area/1/gwendylon.c::reskin_driver`).
pub const CDR_RESKIN: u16 = 20;
/// C `#define CDR_GUIWYNN 21` (`src/system/drvlib.h`, "specific quest
/// giver, area1, city"): the town-mage NPC running the two-part "Order of
/// Mages" investigation quest chain (`src/area/1/gwendylon.c::
/// guiwynn_driver`).
pub const CDR_GUIWYNN: u16 = 21;
/// C `#define CDR_LOGAIN 23` (`src/system/drvlib.h`, "specific quest
/// giver, area1, city"): the retired knight-trainer's mystery-quest
/// dialogue chain (`src/area/1/gwendylon.c::logain_driver`) - the last
/// driver in `ch_driver`'s dispatch table (`gwendylon.c:6076-6155`).
pub const CDR_LOGAIN: u16 = 23;
/// C `#define CDR_CLANMASTER 27` (`src/system/drvlib.h`): the clan
/// foundations NPC (`src/area/30/clanmaster.c::clanmaster_driver`).
pub const CDR_CLANMASTER: u16 = 27;
/// C `#define CDR_CLANCLERK 28` (`src/system/drvlib.h`): the clan
/// administration/treasury NPC (`src/area/30/clanmaster.c::clanclerk_driver`).
pub const CDR_CLANCLERK: u16 = 28;
/// C `#define CDR_CLUBMASTER 113` (`src/system/drvlib.h`): the club
/// foundations/administration NPC (`src/system/clubmaster.c::
/// clubmaster_driver`) - a single driver combining what `CDR_CLANMASTER`/
/// `CDR_CLANCLERK` split into two separate NPCs. See `crate::club`'s
/// module doc comment for the club/clan split, and
/// `crate::world::clubmaster` for the port itself.
pub const CDR_CLUBMASTER: u16 = 113;
/// C `#define CDR_GATE_FIGHT 40` (`src/system/drvlib.h`): the private-room
/// opponent NPC spawned by `enter_room` (`gatekeeper_w`/`gatekeeper_m`/
/// `gatekeeper_s` templates, `src/system/gatekeeper.c::gate_fight_driver`).
pub const CDR_GATE_FIGHT: u16 = 40;
/// C `#define CDR_MILITARY_MASTER 42` (`src/system/drvlib.h`): the
/// mission-giving Military Master NPC (`src/module/military.c::
/// military_master_driver`).
pub const CDR_MILITARY_MASTER: u16 = 42;
/// C `#define CDR_MILITARY_ADVISOR 43` (`src/system/drvlib.h`): the paid
/// mission-recommendation NPC (`src/module/military.c::
/// military_advisor_driver`).
pub const CDR_MILITARY_ADVISOR: u16 = 43;
/// C `#define CDR_ARENAMASTER 48` (`src/system/drvlib.h`): the arena
/// tournament master NPC (`src/system/arena.c::master_driver`) - pairs
/// registered contenders, watches the fight, and scores the result. See
/// the "Arena rankings" P3 task in `PORTING_TODO.md`.
pub const CDR_ARENAMASTER: u16 = 48;
/// C `#define CDR_ARENAFIGHTER 49` (`src/system/drvlib.h`): the
/// autonomous tournament "fighter" bot (`arena.c::fighter_driver`) that
/// registers itself, enters, and fights via the generic `fight_driver_*`
/// helpers (narrowed here to a single tracked enemy, same simplification
/// as `CDR_GATE_FIGHT` - see `world/arena.rs`'s `process_arena_fighter_actions`).
pub const CDR_ARENAFIGHTER: u16 = 49;
/// C `#define CDR_ARENAMANAGER 50` (`src/system/drvlib.h`): the
/// arena-rental NPC (`arena.c::manager_driver`, `rent`/`invite:`/`enter`/
/// `leave` commands - despite the "paid" name, C's own `manager_driver`
/// never touches gold at all). See `world/arena.rs`'s
/// `process_arena_manager_actions`.
pub const CDR_ARENAMANAGER: u16 = 50;
/// C `#define CDR_DUNGEONMASTER 51` (`src/system/drvlib.h`): the clan-raid
/// catacomb reception NPC (`src/area/13/dungeon.c::dungeonmaster`) -
/// `attack <nr>`/`enter <nr>`/`list`/(GM-only) `destroy <nr>` text
/// commands, the per-slot expiry/warning tick, and the greeting. See
/// `world/dungeon_master.rs`'s `process_dungeonmaster_actions`.
pub const CDR_DUNGEONMASTER: u16 = 51;
/// C `#define CDR_DUNGEONFIGHTER 52` (`src/system/drvlib.h`): the
/// autonomous raid-boss combat driver (`dungeon.c::dungeonfighter`/
/// `dungeon_potion`/`fighter_dead`, `dungeon.c:1956-2161`) spawned inside
/// a live catacomb. The message-loop/potion half is ported - see
/// `world/dungeon_fighter.rs`'s `process_dungeonfighter_actions`; its own
/// module doc comment lists what's still REMAINING (the SimpleBaddy-AI
/// tail call and `fighter_dead`).
pub const CDR_DUNGEONFIGHTER: u16 = 52;
/// C `#define CDR_SUPERIOR 26` (`src/system/drvlib.h`, "superior zombie
/// driver in area 2"): the four named crypt guardians
/// (`src/area/2/area2.c::superior_driver`).
pub const CDR_SUPERIOR: u16 = 26;
/// C `#define CDR_MOONIE 31` (`src/system/drvlib.h`, "stop to eat
/// spiders"): the spider-eating companion NPC
/// (`src/area/2/area2.c::moonie_driver`).
pub const CDR_MOONIE: u16 = 31;
/// C `#define CDR_VAMPIRE 34` (`src/system/drvlib.h`, "special NPC in area
/// 2"): the Vampire Lord boss NPC (`src/area/2/area2.c::vampire_driver`).
pub const CDR_VAMPIRE: u16 = 34;
/// C `#define CDR_VAMPIRE2 38` (`src/system/drvlib.h`, "special NPC in
/// area 2"): the Vampire Lord 2 boss NPC
/// (`src/area/2/area2.c::vampire2_driver`).
pub const CDR_VAMPIRE2: u16 = 38;
/// C `#define CDR_ASTRO1 32` (`src/system/drvlib.h`, "tells wild stories"):
/// area 3's ambient moon-telescope astronomer NPC
/// (`src/area/3/area3.c::astro1_driver`).
pub const CDR_ASTRO1: u16 = 32;
/// C `#define CDR_ASTRO2 33` (`src/system/drvlib.h`, "gives moonie-quest
/// in area 2" - a stale comment; the driver itself lives in
/// `src/area/3/area3.c::astro2_driver`): the astronomer whose lost-notes
/// quest (`QLOG` 16) rewards `MONEY_AREA3_MOONIES`.
pub const CDR_ASTRO2: u16 = 33;
/// C `#define CDR_THOMAS 35` (`src/system/drvlib.h`, "gives moonie-quest
/// in area 2" - a stale comment; the driver itself lives in
/// `src/area/3/area3.c::thomas_driver`): the crypt entrance guard who
/// waves in players above level 18.
pub const CDR_THOMAS: u16 = 35;
/// C `#define CDR_SIRJONES 36` (`src/system/drvlib.h`, same stale
/// "gives moonie-quest in area 2" comment as `CDR_THOMAS`): the crypt
/// quest giver (`src/area/3/area3.c::sir_jones_driver`).
pub const CDR_SIRJONES: u16 = 36;
/// C `#define CDR_SEYMOUR 24` (`src/system/drvlib.h`, "specific quest
/// giver, area3, aston"): the Seyan'Du Staff Sergeant who greets new
/// arrivals in Aston and hands out the army-enrollment quest chain
/// (`src/area/3/area3.c::seymour_driver`).
pub const CDR_SEYMOUR: u16 = 24;
/// C `#define CDR_KELLY 30` (`src/system/drvlib.h`, "NPC in area 3"): the
/// Seyan'Du Sergeant who runs area 3's longest quest chain (`src/area/3/
/// area3.c::kelly_driver`).
pub const CDR_KELLY: u16 = 30;
/// C `#define CDR_LAMPGHOST 25` (`src/system/drvlib.h`, "specific lamp
/// extinguisher, area 3, palace"): the palace-light puzzle janitor NPC
/// (`src/area/3/area3.c::lampghost_driver`).
pub const CDR_LAMPGHOST: u16 = 25;
/// C `#define CDR_CARLOS 90` (`src/system/drvlib.h:138`, "gives dragon-
/// breath-quest"): the Imperial Army investigator who runs the dragon-
/// staff quest (quest 20) and the Imperial Vault ritual quest (quest 61)
/// (`src/area/3/area3.c::carlos_driver`).
pub const CDR_CARLOS: u16 = 90;
/// C `#define CDR_SMUGGLECOM 88` (`src/system/drvlib.h:136`, "staffer
/// area: smuggler commander"): the Imperial Commander who runs the
/// Contraband quest chain (quests 35-37) below Aston 2
/// (`src/area/26/staffer.c::smugglecom_driver`).
pub const CDR_SMUGGLECOM: u16 = 88;
/// C `#define CDR_SMUGGLELEAD 89` (`src/system/drvlib.h:137`, "staffer
/// area: smuggler leader"): the final quest-37 kill target. C's own
/// `ch_driver` dispatch (`staffer.c:932-934`) is an unconditional tail
/// call to `char_driver(CDR_SIMPLEBADDY, ...)`, so `CDR_SMUGGLELEAD`
/// characters reuse the SimpleBaddy AI end-to-end - same precedent as
/// `CDR_PENTER`/`CDR_TWOROBBER` (see the `character.driver ==
/// CDR_SIMPLEBADDY` gates widened alongside those in `world/npc_fight.rs`/
/// `world/npc_idle.rs`). Its own `ch_died_driver`/`smugglelead_died` death
/// hook (`staffer.c:658-674`) lives in `ugaris-server`'s
/// `apply_smugglelead_death_from_hurt_event` (needs `PlayerRuntime`'s
/// `staffer_smugglecom_state`).
pub const CDR_SMUGGLELEAD: u16 = 89;
/// C `#define CDR_ROUVEN 130` (`src/system/drvlib.h:180`, "imperial
/// vault"): the Imperial Vault guard who runs quests 62 ("Tunnel Magics")
/// and 63 ("Chronicles of Seyan"), only reachable once Carlos's own
/// ritual quest (quest 61, `carlos2_state`) has been opened
/// (`src/area/26/staffer.c::rouven_driver`).
pub const CDR_ROUVEN: u16 = 130;
/// C `#define CDR_KASSIM 156` (`src/system/drvlib.h:208`, "Aston: Kassim
/// the engraver"): the jewelry engraver (`src/area/3/area3.c::
/// kassim_driver`).
pub const CDR_KASSIM: u16 = 156;
/// C `#define CDR_TUNNELER_GORWIN 158` (`src/system/drvlib.h:210`,
/// "Tunnel Changer NPC"): Gorwin, who runs the Long Tunnels (area 33)
/// entrance lobby and lets players pick their tunnel difficulty level
/// (`src/area/33/tunnel.c::gorwin_driver`).
pub const CDR_TUNNELER_GORWIN: u16 = 158;
/// C `#define CDR_SUPERMAX 98` (`src/system/drvlib.h:146`, "past-maxes-
/// raiser"): the NPC who raises attributes/skills/spells past
/// `skillmax` for a gold+exp fee (`src/area/3/area3.c::supermax_driver`).
pub const CDR_SUPERMAX: u16 = 98;
/// C `#define CDR_WARPFIGHTER 83` (`src/system/drvlib.h:131`, "warped:
/// fighter"): the hired opponent `warptrialdoor_driver`
/// (`src/area/25/warped.c::warptrialdoor_driver`) spawns inside a trial
/// room, self-destructing once its summoning player leaves the room.
pub const CDR_WARPFIGHTER: u16 = 83;
/// C `#define CDR_WARPMASTER 84` (`src/system/drvlib.h:132`, "warped:
/// master"): the key-for-stone trader NPC in area 25's warped world
/// (`src/area/25/warped.c::warpmaster`).
pub const CDR_WARPMASTER: u16 = 84;
/// C `#define CDR_ARISTOCRAT 100` (`src/system/drvlib.h:148`, "brannington
/// forest: robbed noble"): the robbed noble in Brannington Forest who runs
/// "The Family Heirloom" (quest 38)
/// (`src/area/28/brannington_forest.c::aristocrat_driver`).
pub const CDR_ARISTOCRAT: u16 = 100;
/// C `#define CDR_YOATIN 101` (`src/system/drvlib.h:149`, "brannington
/// forest: timid hunter"): the timid hunter in Brannington Forest who runs
/// "Bear Hunt - Again" (quest 39)
/// (`src/area/28/brannington_forest.c::yoatin_driver`).
pub const CDR_YOATIN: u16 = 101;
/// C `#define CDR_SPIRITBRAN 94` (`src/system/drvlib.h:142`, "staffer2
/// area: spirit brannington"): the ghost NPC in Brannington who explains
/// the necromancer plot and runs "The Brannington Holy Relic" (quest 44)
/// (`src/area/29/brannington.c::spirit_brannington_driver`).
pub const CDR_SPIRITBRAN: u16 = 94;
/// C `#define CDR_GUARDBRAN 95` (`src/system/drvlib.h:143`, "staffer2 area:
/// spirit brannington"): the town guard who greets new arrivals, then runs
/// "Finding Arkhata" (quest 64) once Count Brannington's family-heirloom
/// chain (`staffer_ppd.countbran_bits`) is complete
/// (`src/area/29/brannington.c::guard_brannington_driver`).
pub const CDR_GUARDBRAN: u16 = 95;
/// C `#define CDR_FORESTBRAN 97` (`src/system/drvlib.h:145`, "staffer2
/// area: forester brannington"): the Brannington Forest hint giver who
/// decodes thief-mage treasure maps into dig locations. No quest of its own
/// (`src/area/29/brannington.c::forest_brannington_driver`).
pub const CDR_FORESTBRAN: u16 = 97;
/// C `#define CDR_BRENNETHBRAN 96` (`src/system/drvlib.h:144`, "staffer2
/// area: brenneth brannington"): the memory-loss assassin NPC who runs "A
/// Grolm's Spoils"/"A Thief's Loot"/"A Necromancer's Notes" (quests
/// 41-43) (`src/area/29/brannington.c::brenneth_brannington_driver`).
pub const CDR_BRENNETHBRAN: u16 = 96;
/// C `#define CDR_BROKLIN 99` (`src/system/drvlib.h:147`, "Brannington
/// Forest area"): Brannington's Chief Miner, who runs "The Missing
/// Pickaxe"/"The Head Robber" (quests 45/46) and a permanent gold<->silver
/// trade service (`src/area/29/brannington.c::broklin_driver`).
pub const CDR_BROKLIN: u16 = 99;
/// C `#define CDR_COUNTBRAN 91` (`src/system/drvlib.h:139`, "staffer2 area:
/// count brannington"): Count Brannington, who runs "The Jewels of
/// Brannington" (quest 40) and hands out mausoleum keys
/// (`src/area/29/brannington.c::count_brannington_driver`).
pub const CDR_COUNTBRAN: u16 = 91;
/// C `#define CDR_COUNTESSABRAN 92` (`src/system/drvlib.h:140`, "staffer2
/// area: countessa brannington"): the Countessa's secondary quest-40 reward
/// dialogue, gated on `staffer_ppd.countbran_bits`
/// (`src/area/29/brannington.c::countessa_brannington_driver`).
pub const CDR_COUNTESSABRAN: u16 = 92;
/// C `#define CDR_DAUGHTERBRAN 93` (`src/system/drvlib.h:141`, "staffer2
/// area: daughter brannington"): the Daughter's secondary quest-40 reward
/// dialogue, gated on `staffer_ppd.countbran_bits`
/// (`src/area/29/brannington.c::daughter_brannington_driver`).
pub const CDR_DAUGHTERBRAN: u16 = 93;
/// C `#define CDR_WHITEROBBERBOSS 102` (`src/system/drvlib.h:150`,
/// "brannington forest: robber boss"): the final kill target of the
/// Brannington robber camp, whose death (`robberboss_dead`) completes quest
/// 46 ("A Miner's Vengeance") for whichever killer's `broklin_state` sits
/// in `5..=10`. C's own `ch_driver` dispatch (`brannington_forest.c:684-
/// 686`) is an unconditional tail call to `char_driver(CDR_SIMPLEBADDY,
/// ...)`, so `CDR_WHITEROBBERBOSS` characters reuse the SimpleBaddy AI
/// end-to-end - same precedent as `CDR_PENTER`/`CDR_TWOROBBER`/`CDR_
/// SMUGGLELEAD` (see the `character.driver == CDR_SIMPLEBADDY` gates
/// widened alongside those in `world/npc_fight.rs`/`world/npc_idle.rs`,
/// and its `NT_CREATE` zone-spawn wiring in `zone.rs`). Its own `ch_died_
/// driver`/`robberboss_dead` death hook lives in `ugaris-server`'s
/// `apply_robberboss_death_from_hurt_event` (needs `PlayerRuntime`'s
/// `staffer_broklin_state`).
pub const CDR_WHITEROBBERBOSS: u16 = 102;
/// C `#define CDR_CENTINEL 106` (`src/system/drvlib.h:154`, "staffer2 area:
/// centinel"): the wooden marionette sentinels guarding the Brannington
/// tower (`zones/29/wrtower.chr`'s `centinel_count` template). C's own
/// `ch_driver` dispatch (`brannington.c:2802-2804`) is an unconditional
/// tail call to `char_driver(CDR_SIMPLEBADDY, ...)`, so `CDR_CENTINEL`
/// characters reuse the SimpleBaddy AI end-to-end - same precedent as
/// `CDR_WHITEROBBERBOSS` (see the `character.driver == CDR_SIMPLEBADDY`
/// gates widened alongside it in `world/npc_fight.rs`/`world/npc_idle.rs`,
/// and its `NT_CREATE` zone-spawn wiring in `zone.rs`). Its own
/// `ch_died_driver`/`centinel_dead` death hook (`brannington.c:2725-2758`)
/// lives in `ugaris-server`'s `apply_centinel_death_from_hurt_event`
/// (needs `PlayerRuntime`'s `staffer_centinel_count`). Note the *other*
/// `wrtower.chr` template, `centinel` (no `_count` suffix), is plain
/// `driver=7` (`CDR_SIMPLEBADDY`) directly and never reaches this driver
/// id or the kill-counter hook at all - a real, data-driven distinction
/// between the two near-identical templates, not a porting gap.
pub const CDR_CENTINEL: u16 = 106;
/// C `#define CDR_GRINNICH 104` (`src/system/drvlib.h:152`, "staffer2
/// area: grinnich"): the hermit at the entrance of the Brannington tower
/// dungeon who hints at the buried tower and hands adventurers off to
/// Shanra in the basement (`src/area/29/brannington.c::grinnich_driver`).
pub const CDR_GRINNICH: u16 = 104;
/// C `#define CDR_SHANRA 105` (`src/system/drvlib.h:153`, "staffer2 area:
/// shanra"): the storyteller in the Brannington tower dungeon's basement
/// who rewards the tower's sentinel gauntlet with the Grimoire of
/// Animation and teleports adventurers there and back
/// (`src/area/29/brannington.c::shanra_driver`).
pub const CDR_SHANRA: u16 = 105;
/// C `#define CDR_DWARFCHIEF 103` (`src/system/drvlib.h`, "warrmine area:
/// dwarfchief"): Grimroot's leader, who runs "A Miner's Misery"/"A Miner's
/// Bane"/"A Miner's Anguish"/"A Miner Lost" (quests 47-50)
/// (`src/area/31/warrmines.c::dwarfchief_driver`).
pub const CDR_DWARFCHIEF: u16 = 103;
/// C `#define CDR_LOSTDWARF 108` (`src/system/drvlib.h`, "warrmine area:
/// lost miner"): the four (`nr` 1-4) missing miners
/// `dwarfchief_driver`'s quest chain sends the player to rescue
/// (`src/area/31/warrmines.c::lostdwarf_driver`).
pub const CDR_LOSTDWARF: u16 = 108;
/// C `#define CDR_DWARFSHAMAN 109` (`src/system/drvlib.h`, "warrmine area:
/// shaman"): Grimroot's shaman, who runs "Lizard's Teeth"/"Collecting
/// Berries"/"Elitist Head" (quests 51-53)
/// (`src/area/31/warrmines.c::dwarfshaman_driver`).
pub const CDR_DWARFSHAMAN: u16 = 109;
/// C `#define CDR_DWARFSMITH 110` (`src/system/drvlib.h`, "warrmine area:
/// shaman" - comment typo in C, this is actually the blacksmith):
/// Grimroot's blacksmith, who forges a `lizard_elite_keyN` from a mold
/// plus 5,000 silver (`src/area/31/warrmines.c::dwarfsmith_driver`).
pub const CDR_DWARFSMITH: u16 = 110;
/// C `#define CDR_MISSIONGIVE 111` (`src/system/drvlib.h`, "area32:
/// mission giver"): "Mister Jones", the governor's job-board NPC who
/// offers randomly-rolled Alpha/Beta/Gamma kill jobs and runs the
/// brownie-points reward shop (`src/area/32/missions.c::
/// mission_giver_driver`).
pub const CDR_MISSIONGIVE: u16 = 111;
/// C `#define CDR_MISSIONFIGHT 112` (`src/system/drvlib.h`, "mission area:
/// mission giver" - comment typo in C, this is actually the instance-
/// dungeon fighter): every `start_mission`-spawned NPC (easy/normal/hard/
/// boss). C's own `mission_fighter_driver` is an unconditional tail call
/// to `char_driver(CDR_SIMPLEBADDY, ...)` (`missions.c:1849-1851`), same
/// "reuse SimpleBaddy AI wholesale, keep a distinguishable driver id only
/// for the death hook" precedent as `CDR_PENTER`/`CDR_WARPFIGHTER` (see
/// the `character.driver == CDR_SIMPLEBADDY` gates widened alongside this
/// one in `world/npc_fight.rs`/`world/npc_idle.rs`). `ch[cn].deaths`
/// (`Character::deaths` here) doubles as the `fID` fighter-tier tag
/// (`1`=easy/`2`=normal/`3`=hard/`4`=boss) `mission_fighter_dead` reads,
/// per `build_fighter`'s own `ch[cn].deaths = fID` (`missions.c:772`).
pub const CDR_MISSIONFIGHT: u16 = 112;
/// C `#define CDR_SHR_WEREWOLF 86` (`src/system/drvlib.h:134`, "shrike:
/// werewolf"): the invisible-by-day wolf pit guardian in area 38
/// (`src/area/38/shrike.c::shr_werewolf_driver`/`shr_werewolf_dead`).
/// At full night (`moonlight != 0 && sunlight < 100`) it becomes visible
/// and behaves exactly like a plain `CDR_SIMPLEBADDY` (C's unconditional
/// tail call `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
/// lastact)`); by day it stays `CF_INVISIBLE` and walks home. See
/// `world::npc::area38::werewolf` for the day/night driver body and
/// `ugaris-server`'s `apply_shr_werewolf_death_from_hurt_event` for the
/// `shr_werewolf_dead` mist/sprite/`PlayerRuntime::area1_shrike_fails`
/// death hook. Its zone-spawn wiring in `zone.rs` (parsing the same
/// `arg="aggressive=1;helper=0;scavenger=20;..."` string `CDR_SIMPLEBADDY`
/// itself parses) follows the `CDR_WHITEROBBERBOSS`/`CDR_CENTINEL`
/// precedent, but unlike those two pure tail calls, the `CDR_SIMPLEBADDY`
/// gates in `world/npc_fight.rs`/`world/npc_idle.rs` are deliberately
/// *not* widened for this driver - the werewolf's day/night gate must run
/// first (only `world::npc::area38::werewolf` calls the single-character
/// SimpleBaddy action functions directly, only when full night), or it
/// would fight/wander during the day too.
pub const CDR_SHR_WEREWOLF: u16 = 86;
pub const DRD_SIMPLEBADDYDRIVER: u32 = 0x0100_0013;
pub const DRD_CLARADRIVER: u32 = 0x0100_0059;
pub const DRD_SKELLYDRIVER: u32 = 0x0100_006a;
pub const DRD_LAB2_UNDEAD: u32 = 0x0200_0001;
pub const NT_CHAR: i32 = 1;
pub const NT_ITEM: i32 = 2;
pub const NT_GOTHIT: i32 = 3;
pub const NT_DIDHIT: i32 = 4;
pub const NT_SEEHIT: i32 = 5;
pub const NT_DEAD: i32 = 6;
pub const NT_SPELL: i32 = 7;
pub const NT_GIVE: i32 = 8;
pub const NT_CREATE: i32 = 9;
pub const NT_TEXT: i32 = 200;
pub const NT_NPC: i32 = 300;
pub const NTID_MERCHANT: i32 = 1;
pub const NTID_TERION: i32 = 2;
pub const NTID_ASTURIN: i32 = 3;
pub const NTID_GATEKEEPER: i32 = 4;
pub const NTID_DIDSAY: i32 = 5;
pub const NTID_TUTORIAL: i32 = 6;
pub const NTID_PALACE_ALERT: i32 = 7;
pub const NTID_ARENA: i32 = 8;
pub const NTID_DUNGEON: i32 = 9;
pub const NTID_TWOCITY: i32 = 10;
pub const NTID_TWOCITY_PICK: i32 = 11;
pub const NTID_DICE: i32 = 12;
pub const NTID_LABGNOMETORCH: i32 = 13;
pub const NTID_LAB2_DEAMONCHECK: i32 = 14;
pub const NTID_SALTMINE_USEITEM: i32 = 15;
pub const NTID_GLADIATOR: i32 = 16;
pub const NTID_FDEMON: i32 = 17;
pub const FDEMON_MSG_WAYPOINT: i32 = 1;
