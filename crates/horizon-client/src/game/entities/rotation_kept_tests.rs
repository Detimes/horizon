//! `Entity.load` keeps a saved `Rotation` on falling blocks, shulkers and area
//! effect clouds, and `/data get` reads it back (#16909).
use super::commands::Target;
use crate::game::native_entity_refs::tests::with_game;
use horizon_core::nbt::Tag;

#[test]
#[ignore = "daily"]
fn a_summoned_falling_block_shulker_and_cloud_keep_their_rotation() {
    with_game(|game| {
        game.build_registries();
        game.set_allow_commands(true);
        game.summon_with_nbt(
            "minecraft:falling_block",
            [0.5, 70.0, 0.5],
            r#"{Rotation:[75f,20f],BlockState:{Name:"minecraft:sand"},Time:1}"#,
        )
        .unwrap();
        game.summon_with_nbt(
            "minecraft:shulker",
            [4.5, 70.0, 0.5],
            "{Rotation:[75f,20f]}",
        )
        .unwrap();
        game.summon_with_nbt(
            "minecraft:area_effect_cloud",
            [8.5, 70.0, 0.5],
            "{Rotation:[75f,20f]}",
        )
        .unwrap();
        let targets = [
            ("falling block", Target::Falling(game.falling[0].id)),
            ("shulker", Target::Mob(game.mobs[0].id)),
            ("cloud", Target::Cloud(game.breath_clouds[0].id)),
        ];
        for (name, target) in targets {
            let tag = game.command_target_nbt(target).unwrap();
            let Some(Tag::List(rotation)) = tag.get("Rotation") else {
                panic!("{name}: no Rotation in {tag:?}");
            };
            assert_eq!(
                rotation.as_slice(),
                [
                    Tag::Float(75.0_f32.to_bits()),
                    Tag::Float(20.0_f32.to_bits())
                ],
                "{name}"
            );
        }
    });
}
