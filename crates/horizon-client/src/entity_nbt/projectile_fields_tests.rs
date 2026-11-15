use crate::entity_nbt::{arrow_from_nbt, arrow_to_nbt, orb_to_nbt, orbs_from_nbt};
use horizon_world::nbt::{Tag, parse_snbt};

/// `Entity.load` reads `TicksFrozen` and `saveWithoutId` writes it back while positive.
#[test]
fn arrow_and_orb_keep_their_saved_ticks_frozen() {
    let parse = |text: &str| parse_snbt(text).unwrap().0;
    let raw = parse("{id:\"minecraft:arrow\",Pos:[1d,200d,1d],TicksFrozen:60}");
    let (arrow, _) = arrow_from_nbt(&raw, None, |_| None).unwrap();
    assert_eq!(arrow.ticks_frozen, 60);
    let saved = arrow_to_nbt(&arrow, [197, 0, 0, 1], None, None, Some(&raw)).unwrap();
    assert_eq!(saved.get("TicksFrozen"), Some(&Tag::Int(60)));
    let (fresh, _) = arrow_from_nbt(
        &parse("{id:\"minecraft:arrow\",Pos:[1d,200d,1d]}"),
        None,
        |_| None,
    )
    .unwrap();
    assert_eq!(fresh.ticks_frozen, 0);
    let orb = orbs_from_nbt(&parse("{Pos:[1d,200d,1d],TicksFrozen:60}")).remove(0);
    assert_eq!(orb.ticks_frozen, 60);
    assert_eq!(
        orb_to_nbt(&orb, [197, 0, 0, 1]).get("TicksFrozen"),
        Some(&Tag::Int(60))
    );
    let cold = orbs_from_nbt(&parse("{Pos:[1d,200d,1d]}")).remove(0);
    assert!(
        orb_to_nbt(&cold, [197, 0, 0, 1])
            .get("TicksFrozen")
            .is_none()
    );
}

/// `Entity.load` reads `Air` and `saveWithoutId` writes it back.
#[test]
fn arrow_keeps_its_saved_air() {
    let parse = |text: &str| parse_snbt(text).unwrap().0;
    let raw = parse("{id:\"minecraft:arrow\",Pos:[1d,200d,1d],Air:32767s,Fire:7s}");
    let (arrow, _) = arrow_from_nbt(&raw, None, |_| None).unwrap();
    assert_eq!(arrow.air_supply, 32767);
    let saved = arrow_to_nbt(&arrow, [197, 0, 0, 1], None, None, Some(&raw)).unwrap();
    assert_eq!(saved.get("Air"), Some(&Tag::Short(32767)));
    assert_eq!(saved.get("Fire"), Some(&Tag::Short(7)));
    let absent = parse("{id:\"minecraft:arrow\",Pos:[1d,200d,1d]}");
    let (fresh, _) = arrow_from_nbt(&absent, None, |_| None).unwrap();
    assert_eq!(fresh.air_supply, 300);
}
