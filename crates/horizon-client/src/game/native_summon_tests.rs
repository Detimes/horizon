//! `native_summon` and `native_summon_with_uuid` build an entity tree from data as `/summon` does, and
//! `native_entity_merge_nbt` changes a living entity as `/data merge entity` does and refuses players.
use super::super::*;
use crate::game::native_entities::{DisplayTransformation, NativeDisplay};
use horizon_client::mod_api::NativeAuthority;
use horizon_world::nbt::{Compound, Tag};

fn game() -> Game<'static> {
    let root = std::path::PathBuf::from(
        std::env::var_os("HORIZON_TEST_RESOURCES")
            .expect("set HORIZON_TEST_RESOURCES to prepared resources"),
    );
    let settings = Settings::default();
    let loaded = load_resources(
        root.clone(),
        None,
        &root,
        &settings,
        std::time::Instant::now(),
        false,
        true,
        &Progress::new(),
        &mut |_, _| unreachable!(),
    )
    .unwrap();
    let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
    options.resources = root.clone();
    options.execution_role = execution_role::ExecutionRole::IntegratedServer;
    options.dedicated = Some(Vec::new());
    let res: &'static _ = Box::leak(Box::new(loaded.resources));
    let mut game = Game::new(
        res,
        loaded.pack,
        loaded.gui.unwrap(),
        options,
        settings,
        Audio::silent(&root),
        None,
    )
    .unwrap();
    game.build_registries();
    game.set_allow_commands(true);
    game.native_lifecycle
        .begin_open(NativeAuthority::IntegratedServer);
    game.world.chunks.insert(
        (0, 0),
        std::sync::Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
    );
    game.block_ticks
        .add_empty_chunk(horizon_core::position::ChunkPos::new(0, 0))
        .unwrap();
    game
}

fn compound(kind: &str) -> Compound {
    let mut c = Compound::new();
    c.insert("id".into(), Tag::String(kind.into()));
    c
}

fn pig_with_chicken() -> Compound {
    let mut pig = Compound::new();
    pig.insert(
        "Passengers".into(),
        Tag::List(vec![Tag::Compound(compound("minecraft:chicken"))]),
    );
    pig
}

const AT: [f64; 3] = [4.5, 200.0, 4.5];

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_summoned_passenger_exists_and_rides() {
    let mut game = game();
    let dim = game.dim.id();
    let root = game
        .native_summon(dim, "minecraft:pig", AT, pig_with_chicken())
        .unwrap();
    assert_eq!(game.mobs.len(), 2);
    let rider = game.mobs.iter().find(|m| m.id != root).unwrap();
    assert_eq!(rider.extra.mount, Some(root));
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_chosen_uuid_is_used_and_a_taken_one_is_refused() {
    let mut game = game();
    let dim = game.dim.id();
    let uuid = 0x1234_5678_9abc_def0_1234_5678_9abc_def0_u128;
    let id = game
        .native_summon_with_uuid(dim, "minecraft:pig", AT, pig_with_chicken(), uuid)
        .unwrap();
    assert_eq!(
        game.native_entity_by_uuid(uuid),
        Some((dim.to_owned(), id, "minecraft:pig".to_owned()))
    );
    assert_eq!(game.mobs.len(), 2);
    // The UUID is in use now: nothing more spawns.
    assert_eq!(
        game.native_summon_with_uuid(dim, "minecraft:cow", AT, Compound::new(), uuid),
        None
    );
    assert_eq!(game.mobs.len(), 2);
    // A passenger that takes the UUID of an entity of the level refuses the tree.
    let mut cow = Compound::new();
    let mut rider = compound("minecraft:chicken");
    rider.insert(
        "UUID".into(),
        Tag::IntArray(horizon_client::lan::playerdata::uuid_ints(uuid).to_vec()),
    );
    cow.insert("Passengers".into(), Tag::List(vec![Tag::Compound(rider)]));
    assert_eq!(
        game.native_summon_with_uuid(dim, "minecraft:cow", AT, cow, uuid + 1),
        None
    );
    assert_eq!(game.mobs.len(), 2);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_merge_changes_a_field_and_a_player_is_refused() {
    let mut game = game();
    let dim = game.dim.id();
    let id = game
        .native_summon(dim, "minecraft:item_frame", AT, {
            let mut c = Compound::new();
            c.insert("Facing".into(), Tag::Byte(3));
            c
        })
        .unwrap();
    let entity = game.native_entity(id).unwrap();
    let mut merge = Compound::new();
    merge.insert("ItemRotation".into(), Tag::Byte(5));
    assert!(game.native_entity_merge_nbt(entity, merge.clone()));
    assert_eq!(
        game.native_entity_nbt(entity)
            .unwrap()
            .get(&"ItemRotation".into()),
        Some(&Tag::Byte(5))
    );
    // Merging what is there already changes nothing.
    assert!(!game.native_entity_merge_nbt(entity, merge));
    // `/data merge entity` can't modify a player.
    assert!(
        game.apply_entity_nbt(commands::Target::Player, Tag::Compound(Compound::new()))
            .is_err()
    );
}

/// A transformation tag scaling by `s` on every axis.
fn transformation_with_scale(s: f32) -> Tag {
    DisplayTransformation::from(&NativeDisplay {
        scale: [s; 3],
        ..NativeDisplay::default()
    })
    .to_tag()
}

fn nbt_of(game: &mut Game<'_>, id: u32) -> Compound {
    let entity = game.native_entity(id).unwrap();
    game.native_entity_nbt(entity).unwrap()
}

fn merged(pairs: &[(&str, Tag)]) -> Compound {
    let mut c = Compound::new();
    for (k, v) in pairs {
        c.insert((*k).into(), v.clone());
    }
    c
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_merge_changes_a_text_display_text_and_transformation() {
    let mut game = game();
    let dim = game.dim.id();
    let id = game
        .native_summon(
            dim,
            "minecraft:text_display",
            AT,
            merged(&[("text", Tag::String("old".into()))]),
        )
        .unwrap();
    let entity = game.native_entity(id).unwrap();
    let text = merged(&[
        ("text", Tag::String("new".into())),
        ("transformation", transformation_with_scale(2.0).clone()),
    ]);
    assert!(game.native_entity_merge_nbt(entity, text));
    let nbt = nbt_of(&mut game, id);
    assert_eq!(nbt.get(&"text".into()), Some(&Tag::String("new".into())));
    assert_eq!(
        nbt.get(&"transformation".into()),
        Some(&transformation_with_scale(2.0))
    );
    // The saved data is what the display is shown with: the raw compound holds the merge.
    assert_eq!(
        game.persist.raw.get(&id).and_then(|t| t.get("text")),
        Some(&Tag::String("new".into()))
    );
    // Merging what is there already changes nothing.
    assert!(!game.native_entity_merge_nbt(entity, merged(&[("text", Tag::String("new".into()))])));
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_merge_changes_an_item_display_transformation_used_on_save() {
    let mut game = game();
    let dim = game.dim.id();
    let id = game
        .native_summon(dim, "minecraft:item_display", AT, Compound::new())
        .unwrap();
    let entity = game.native_entity(id).unwrap();
    assert!(game.native_entity_merge_nbt(
        entity,
        merged(&[("transformation", transformation_with_scale(3.0))])
    ));
    // The saved form takes the transformation from the state the display is shown with.
    assert_eq!(
        nbt_of(&mut game, id).get(&"transformation".into()),
        Some(&transformation_with_scale(3.0))
    );
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_merge_changes_an_interaction_size() {
    let mut game = game();
    let dim = game.dim.id();
    let id = game
        .native_summon(
            dim,
            "minecraft:interaction",
            AT,
            merged(&[
                ("width", Tag::Float(1.0f32.to_bits())),
                ("height", Tag::Float(1.0f32.to_bits())),
            ]),
        )
        .unwrap();
    let entity = game.native_entity(id).unwrap();
    assert!(game.native_entity_merge_nbt(
        entity,
        merged(&[
            ("width", Tag::Float(2.5f32.to_bits())),
            ("height", Tag::Float(3.0f32.to_bits()))
        ]),
    ));
    let nbt = nbt_of(&mut game, id);
    assert_eq!(
        nbt.get(&"width".into()),
        Some(&Tag::Float(2.5f32.to_bits()))
    );
    assert_eq!(
        nbt.get(&"height".into()),
        Some(&Tag::Float(3.0f32.to_bits()))
    );
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_merge_into_a_marker_keeps_its_data() {
    let mut game = game();
    let dim = game.dim.id();
    let id = game
        .native_summon(
            dim,
            "minecraft:marker",
            AT,
            merged(&[("data", Tag::Compound(merged(&[("a", Tag::Int(1))])))]),
        )
        .unwrap();
    let entity = game.native_entity(id).unwrap();
    assert!(game.native_entity_merge_nbt(
        entity,
        merged(&[(
            "Rotation",
            Tag::List(vec![Tag::Float(90f32.to_bits()), Tag::Float(0)])
        )]),
    ));
    let nbt = nbt_of(&mut game, id);
    assert_eq!(
        nbt.get(&"data".into()),
        Some(&Tag::Compound(merged(&[("a", Tag::Int(1))])))
    );
    assert_eq!(game.native_level_markers(dim).len(), 1);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn summon_takes_markers_and_interactions_with_nbt() {
    let mut game = game();
    let dim = game.dim.id();
    let marker = game
        .native_summon(
            dim,
            "minecraft:marker",
            AT,
            merged(&[("data", Tag::Compound(merged(&[("k", Tag::Int(7))])))]),
        )
        .expect("marker summons");
    assert_eq!(
        nbt_of(&mut game, marker).get(&"data".into()),
        Some(&Tag::Compound(merged(&[("k", Tag::Int(7))])))
    );
    let interaction = game
        .native_summon(
            dim,
            "minecraft:interaction",
            AT,
            merged(&[("width", Tag::Float(0.5f32.to_bits()))]),
        )
        .expect("interaction summons");
    assert_eq!(
        nbt_of(&mut game, interaction).get(&"width".into()),
        Some(&Tag::Float(0.5f32.to_bits()))
    );
}
