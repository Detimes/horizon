//! `player_send_entity_data`: one viewer is sent different data for an entity than the others.
use super::Game;
use horizon_protocol::{
    entity_data::{Value, serializer},
    packets::play::{self, Clientbound as C},
};
use mod_api::{EntityRef, EntityViewData, EntityViewError as Error, PlayerRef, Pose};

/// `Pose.id()` as `EntityDataSerializers.POSE` writes it (the `Pose` enum's order).
fn pose_id(pose: Pose) -> i32 {
    match pose {
        Pose::Standing => 0,
        Pose::FallFlying => 1,
        Pose::Sleeping => 2,
        Pose::Swimming => 3,
        Pose::SpinAttack => 4,
        Pose::Crouching => 5,
        Pose::LongJumping => 6,
        Pose::Dying => 7,
        Pose::Croaking => 8,
        Pose::UsingTongue => 9,
        Pose::Sitting => 10,
        Pose::Roaring => 11,
        Pose::Sniffing => 12,
        Pose::Emerging => 13,
        Pose::Digging => 14,
        Pose::Sliding => 15,
        Pose::Shooting => 16,
        Pose::Inhaling => 17,
    }
}

impl Game<'_> {
    /// The entity's data entries as `data` names them, ordered by index as the client reads
    /// them, or why there are none.
    fn entity_view_values(&self, data: &EntityViewData) -> Result<Vec<(u8, i32, Value)>, Error> {
        if data.custom_name.is_some() && data.remove_custom_name {
            return Err(Error::Conflict);
        }
        let mut values = Vec::new();
        if let Some(flags) = data.flags {
            values.push((0, serializer::BYTE, Value::Byte(flags as i8)));
        }
        if let Some(name) = &data.custom_name {
            let tag = self.text_to_nbt(name).map_err(Error::Text)?;
            values.push((2, serializer::OPTIONAL_COMPONENT, Value::OptText(Some(tag))));
        } else if data.remove_custom_name {
            values.push((2, serializer::OPTIONAL_COMPONENT, Value::OptText(None)));
        }
        if let Some(visible) = data.custom_name_visible {
            values.push((3, serializer::BOOLEAN, Value::Bool(visible)));
        }
        if let Some(pose) = data.pose {
            values.push((6, serializer::POSE, Value::Id(pose_id(pose))));
        }
        if values.is_empty() {
            return Err(Error::Empty);
        }
        Ok(values)
    }

    /// Sends the viewer's client one `SetEntityData` for the entity, with the server's state
    /// untouched.
    pub(super) fn player_send_entity_data(
        &mut self,
        viewer: PlayerRef,
        entity: EntityRef,
        data: EntityViewData,
    ) -> Result<(), Error> {
        if !self.is_logical_server() {
            return Err(Error::Client);
        }
        let epoch = self.native_lifecycle.epoch();
        if epoch != Some(viewer.entity().epoch()) {
            return Err(Error::Stale);
        }
        let reachable = self.native_acting_player() == Some(viewer)
            || self.online_player_record(viewer).is_some();
        if !reachable {
            return Err(Error::Stale);
        }
        if epoch != Some(entity.epoch()) || self.native_entity_target(entity).is_none() {
            return Err(Error::StaleEntity);
        }
        let id = i32::try_from(entity.id()).map_err(|_| Error::StaleEntity)?;
        let values = self.entity_view_values(&data)?;
        let packet = C::SetEntityData(play::SetEntityData { id, values });
        if self.native_send(viewer.uuid(), packet) {
            Ok(())
        } else {
            Err(Error::NoConnection)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_guest_turn_tests::with_guests;
    use super::*;
    use horizon_client::lan::{Out, PlayerSender};
    use mod_api::Text;
    use std::sync::{Arc, atomic::AtomicUsize, mpsc::Receiver};

    /// Everything sent to the guest since the last call.
    fn sent(rx: &Receiver<Out>) -> Vec<C> {
        rx.try_iter()
            .filter_map(|out| match out {
                Out::Packet(body) => {
                    let mut r = horizon_protocol::buf::Reader::new(&body);
                    let id = r.varint().ok()?;
                    C::decode(id, &body[r.position()..]).ok()
                }
                _ => None,
            })
            .collect()
    }

    /// The pose ids are the `Pose` enum's order, as the client reads them.
    #[test]
    fn a_pose_goes_out_as_its_ordinal() {
        for (pose, id) in [
            (Pose::Standing, 0),
            (Pose::Sleeping, 2),
            (Pose::Crouching, 5),
            (Pose::Sitting, 10),
            (Pose::Inhaling, 17),
        ] {
            assert_eq!(pose_id(pose), id);
        }
    }

    /// One viewer is sent one packet with the entries in index order; a bad call sends nothing.
    #[test]
    #[ignore = "daily"]
    fn the_viewer_alone_is_sent_the_entity_data() {
        with_guests(|game| {
            let (tx, rx) = std::sync::mpsc::channel();
            game.lan
                .as_mut()
                .unwrap()
                .hub
                .update_guest(12, |g| {
                    g.sender = PlayerSender::new(tx, Arc::new(AtomicUsize::new(0)));
                })
                .unwrap();
            let (viewer, other) = (game.player(12).unwrap(), game.player(13).unwrap());
            let mut data = EntityViewData::new();
            data.custom_name = Some(Text::literal("Hello"));
            data.custom_name_visible = Some(true);
            data.flags = Some(0x20);
            data.pose = Some(Pose::Sleeping);
            assert_eq!(
                game.player_send_entity_data(viewer, other.entity(), data.clone()),
                Ok(())
            );
            let packets = sent(&rx);
            let [C::SetEntityData(packet)] = &packets[..] else {
                panic!("{packets:?}");
            };
            assert_eq!(packet.id as u32, other.entity().id());
            assert_eq!(
                packet.values,
                vec![
                    (0, 0, Value::Byte(0x20)),
                    (
                        2,
                        6,
                        Value::OptText(Some(horizon_core::nbt::Tag::String("Hello".into())))
                    ),
                    (3, 8, Value::Bool(true)),
                    (6, 20, Value::Id(2)),
                ]
            );
            data.custom_name = None;
            data.remove_custom_name = true;
            assert_eq!(
                game.player_send_entity_data(viewer, other.entity(), data),
                Ok(())
            );
            let removed = sent(&rx);
            assert!(
                matches!(&removed[..], [C::SetEntityData(p)]
                    if p.values[1] == (2, 6, Value::OptText(None))),
                "{removed:?}"
            );
            assert_eq!(
                game.player_send_entity_data(viewer, other.entity(), EntityViewData::new()),
                Err(Error::Empty)
            );
            let mut both = EntityViewData::new();
            both.custom_name = Some(Text::literal("x"));
            both.remove_custom_name = true;
            assert_eq!(
                game.player_send_entity_data(viewer, other.entity(), both),
                Err(Error::Conflict)
            );
            assert!(sent(&rx).is_empty(), "a refused call sends nothing");
        });
    }
}
