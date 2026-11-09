use super::*;
use crate::nbt::parse_snbt;

fn snbt(s: &str) -> Tag {
    parse_snbt(s).unwrap().0
}

#[test]
fn current_data_is_left_alone() {
    let mut tag = snbt("{DataVersion: 5023, Status: \"minecraft:full\"}");
    let before = tag.clone();
    update_chunk(&mut tag, "minecraft:overworld", Some("minecraft:noise"));
    assert_eq!(tag, before);
}

#[test]
fn item_stacks_of_1_20_take_components() {
    let mut player = snbt(
        "{DataVersion: 3465, Inventory: [{Slot: 0b, id: \"minecraft:diamond_sword\", Count: 1b, \
         tag: {Damage: 5, display: {Name: '{\"text\":\"Blade\"}'}}}]}",
    );
    update_player(&mut player);
    let stack = &dynamic::stream(player.get("Inventory"))[0];
    assert_eq!(
        dynamic::as_str(stack.get("id")).as_deref(),
        Some("minecraft:diamond_sword")
    );
    assert_eq!(stack.get("count"), Some(&Tag::Int(1)));
    let components = stack.get("components").expect("components");
    assert_eq!(components.get("minecraft:damage"), Some(&Tag::Int(5)));
    assert_eq!(
        components.get("minecraft:custom_name"),
        Some(&Tag::compound([("text", dynamic::string("Blade"))]))
    );
}

#[test]
fn level_data_of_1_20_moves_to_the_new_fields() {
    let mut level = snbt(
        "{DataVersion: 3465, Difficulty: 3b, DifficultyLocked: 1b, hardcore: 0b, SpawnX: 1, SpawnY: 64, SpawnZ: -3, \
         SpawnAngle: 90.0f, GameRules: {doFireTick: \"false\", keepInventory: \"true\", randomTickSpeed: \"5\"}}",
    );
    update(Ty::Level, &mut level, 3465, CURRENT);
    let settings = level
        .get("difficulty_settings")
        .expect("difficulty settings");
    assert_eq!(
        dynamic::as_str(settings.get("difficulty")).as_deref(),
        Some("hard")
    );
    assert_eq!(settings.get("locked"), Some(&Tag::Byte(1)));
    let rules = level.get("game_rules").expect("game rules");
    assert_eq!(rules.get("minecraft:keep_inventory"), Some(&Tag::Byte(1)));
    assert_eq!(rules.get("minecraft:random_tick_speed"), Some(&Tag::Int(5)));
    assert_eq!(
        rules.get("minecraft:fire_spread_radius_around_player"),
        Some(&Tag::Int(0))
    );
    assert_eq!(
        level.get("spawn").and_then(|s| s.get("pos")),
        Some(&Tag::IntArray(vec![1, 64, -3]))
    );
}

#[test]
fn block_states_take_their_new_field_names() {
    let mut chunk = snbt(
        "{DataVersion: 4903, Status: \"minecraft:full\", sections: [{Y: 0b, block_states: \
         {palette: [{Name: \"minecraft:chain\", Properties: {axis: \"y\"}}]}}]}",
    );
    update_chunk(&mut chunk, "minecraft:the_nether", Some("minecraft:noise"));
    let section = &dynamic::stream(chunk.get("sections"))[0];
    let state = &dynamic::stream(section.get("block_states").and_then(|b| b.get("palette")))[0];
    assert_eq!(
        dynamic::as_str(state.get("id")).as_deref(),
        Some("minecraft:chain")
    );
    assert!(state.get("properties").is_some());
    assert_eq!(chunk.get("DataVersion"), Some(&Tag::Int(CURRENT)));
}

#[test]
fn block_states_from_the_flattening_on_are_not_renamed() {
    if !tables::available() {
        eprintln!("skipping: no prepared datafix tables");
        return;
    }
    let props = [("facing".to_owned(), "north".to_owned())];
    assert_eq!(
        native_upgrade_block_state("minecraft:oak_stairs", &props, FLATTENING),
        ("minecraft:oak_stairs".to_owned(), props.to_vec())
    );
    assert_eq!(
        native_upgrade_block_state("minecraft:no_such_block", &[], 1400),
        ("minecraft:no_such_block".to_owned(), Vec::new())
    );
}

#[test]
fn block_states_from_before_the_flattening_take_their_new_names() {
    if !tables::available() {
        eprintln!("skipping: no prepared datafix tables");
        return;
    }
    let granite = [("variant".to_owned(), "granite".to_owned())];
    assert_eq!(
        native_upgrade_block_state("minecraft:stone", &granite, 1400),
        ("minecraft:granite".to_owned(), Vec::new())
    );
    // The same state written after the flattening is not upgraded.
    assert_eq!(
        native_upgrade_block_state("minecraft:stone", &granite, FLATTENING),
        ("minecraft:stone".to_owned(), granite.to_vec())
    );
}

const SIGN: &str = "{id: \"minecraft:sign\", x: 1, y: 64, z: 2, Text1: '{\"text\":\"north\"}', Text2: '\"\"', \
                    Text3: '\"\"', Text4: '\"\"', Color: \"black\", GlowingText: 0b}";
const VILLAGER: &str = "{id: \"minecraft:villager\", Pos: [0.5d, 64.0d, 0.5d], Profession: 1, Career: 2, \
                        CareerLevel: 1, Health: 20.0f}";

fn compound(tag: Tag) -> Compound {
    match tag {
        Tag::Compound(c) => c,
        other => panic!("expected a compound, got {other:?}"),
    }
}

#[test]
fn native_block_entity_matches_the_chunk_loader() {
    // A sign of DataVersion 3400: its four text lines become `front_text`, as the chunk loader does.
    let old = 3400;
    let sign = snbt(SIGN);
    let mut chunk = Tag::compound([
        ("DataVersion", Tag::Int(old)),
        ("Status", dynamic::string("minecraft:full")),
        ("block_entities", Tag::List(vec![sign.clone()])),
    ]);
    update_chunk(&mut chunk, "minecraft:overworld", None);
    let through_chunk = &dynamic::stream(chunk.get("block_entities"))[0];
    assert!(through_chunk.get("front_text").is_some());
    assert!(through_chunk.get("Text1").is_none());

    let native = native_upgrade_block_entity(old, compound(sign));
    assert_eq!(&Tag::Compound(native), through_chunk);
}

#[test]
fn native_entity_matches_the_entity_chunk_loader() {
    // A villager of DataVersion 1900: profession and career become `VillagerData`, as the entity chunk loader does.
    let old = 1900;
    let villager = snbt(VILLAGER);
    let mut record = Tag::compound([
        ("DataVersion", Tag::Int(old)),
        ("Entities", Tag::List(vec![villager.clone()])),
    ]);
    update_region_tag(Ty::EntityChunk, &mut record);
    let through_chunk = &dynamic::stream(record.get("Entities"))[0];
    assert!(through_chunk.get("VillagerData").is_some());
    assert!(through_chunk.get("Profession").is_none());

    let native = native_upgrade_entity(old, compound(villager));
    assert_eq!(&Tag::Compound(native), through_chunk);
}

#[test]
fn native_upgrades_leave_current_and_unknown_data_alone() {
    let sign = compound(snbt(SIGN));
    assert_eq!(native_upgrade_block_entity(CURRENT, sign.clone()), sign);
    let unknown = compound(snbt(
        "{id: \"minecraft:no_such_block_entity\", Text1: '\"x\"', x: 0, y: 0, z: 0}",
    ));
    assert_eq!(native_upgrade_block_entity(3400, unknown.clone()), unknown);
}
