//! A player's skin for server mods (SkinsRestorer): `PlayerRef::set_profile_property` changes
//! their profile, `PlayerRef::refresh` sends them to the other players again.
use super::Game;
use horizon_protocol::packets::login::Property;
use mod_api::PlayerRef;

/// `ExtraCodecs.PROPERTY_MAP`'s limits: 16 properties, names of 64
/// characters, values of 32767 and signatures of 1024. The wire counts UTF-16 code units
/// (`FriendlyByteBuf.writeUtf` and the reader's `readUtf` limits), so a character past the
/// basic plane counts twice.
const MAX_PROPERTIES: usize = 16;
const MAX_PROPERTY_NAME: usize = 64;
const MAX_PROPERTY_VALUE: usize = 32767;
const MAX_PROPERTY_SIGNATURE: usize = 1024;

/// A string's length in UTF-16 code units, what the wire limits count.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `properties` with `property` in place of the one of its name, or added; None when the profile
/// already has 16 and the name is new.
fn with_property(mut properties: Vec<Property>, property: Property) -> Option<Vec<Property>> {
    if let Some(old) = properties.iter_mut().find(|p| p.name == property.name) {
        *old = property;
    } else if properties.len() < MAX_PROPERTIES {
        properties.push(property);
    } else {
        return None;
    }
    Some(properties)
}

impl Game<'_> {
    /// Sets property `name` of the player's profile, replacing one of the same name.
    pub(super) fn player_set_profile_property(
        &mut self,
        player: PlayerRef,
        name: &str,
        value: &str,
        signature: Option<&str>,
    ) -> bool {
        if name.is_empty()
            || utf16_len(name) > MAX_PROPERTY_NAME
            || utf16_len(value) > MAX_PROPERTY_VALUE
            || signature.is_some_and(|s| utf16_len(s) > MAX_PROPERTY_SIGNATURE)
        {
            return false;
        }
        let Some(i) = self.connected_guest_index(player) else {
            return false;
        };
        let Some(host) = self.lan.as_mut() else {
            return false;
        };
        let property = Property {
            name: name.to_owned(),
            value: value.to_owned(),
            signature: signature.map(str::to_owned),
        };
        // The profile as the player list gives it now (the integrated server's owner has the account's
        // properties until a mod sets one), so one property added does not take the others away.
        let Some(updated) = with_property(host.hub.listed_profile_properties(i), property) else {
            return false;
        };
        host.hub.guests.update(i, |g| g.properties = updated);
        true
    }

    /// The index of a connected player's record in the hub's guest list.
    fn connected_guest_index(&self, player: PlayerRef) -> Option<usize> {
        let conn = self.online_player_record(player)?.conn;
        self.lan.as_ref()?.hub.guests.connection_index(conn)
    }

    /// Sends the player to every other player again (`Hub::refresh_player`).
    pub(super) fn player_refresh(&mut self, player: PlayerRef) -> bool {
        let Some(i) = self.connected_guest_index(player) else {
            return false;
        };
        let Some(host) = self.lan.as_mut() else {
            return false;
        };
        let uuid = host.hub.guests[i].uuid;
        host.hub.refresh_player(uuid);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(name: &str, value: &str) -> Property {
        Property {
            name: name.to_owned(),
            value: value.to_owned(),
            signature: None,
        }
    }

    /// The wire counts UTF-16 units: a character past the basic plane is two.
    #[test]
    fn lengths_are_counted_in_utf16_units() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("\u{1F600}"), 2);
        assert!(utf16_len(&"\u{1F600}".repeat(33)) > MAX_PROPERTY_NAME);
        assert!("\u{1F600}".repeat(32).encode_utf16().count() <= MAX_PROPERTY_NAME);
    }

    /// A property of a new name is added next to the others (the owner's skin among them), one of the
    /// same name replaces, and a full profile refuses a new name only.
    #[test]
    fn a_property_is_added_beside_the_others_and_the_sixteenth_is_the_last() {
        let skin = vec![property("textures", "owner")];
        let added = with_property(skin.clone(), property("example:badge", "1")).unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(added[0].value, "owner", "the skin stays");
        let replaced = with_property(added, property("textures", "mod")).unwrap();
        assert_eq!(replaced.len(), 2);
        assert_eq!(replaced[0].value, "mod");
        let full: Vec<_> = (0..MAX_PROPERTIES)
            .map(|i| property(&format!("p{i}"), "v"))
            .collect();
        assert!(with_property(full.clone(), property("new", "v")).is_none());
        assert_eq!(with_property(full, property("p3", "w")).unwrap()[3].value, "w");
    }
}
