//! `player_set_sidebar`: a sidebar only one player's client holds.
use super::{Game, api_values::TryIntoEngine, native_server::NativeSidebar};
use mod_api::{PlayerRef, Sidebar, TextFormError};

/// The lines of a sidebar the client shows (`Objective` scores sorted, the 15 highest).
const SIDEBAR_LINES: usize = 15;
/// The deepest NBT the packet writer takes.
const NBT_DEPTH: usize = 512;

impl Game<'_> {
    /// Shows `sidebar` to that player, or takes it away (None). Ok(false) when they
    /// aren't on, whatever the text; for a player who is on, a text the boundary refuses
    /// (also one whose NBT form is deeper than the writer takes) changes nothing. Only
    /// the first 15 lines are read: the rest are never shown.
    pub(super) fn player_set_sidebar(
        &mut self,
        player: PlayerRef,
        sidebar: Option<Sidebar>,
    ) -> Result<bool, TextFormError> {
        let online = self.native_acting_player() == Some(player)
            || self.online_player_record(player).is_some();
        if !online {
            return Ok(false);
        }
        let sidebar = match sidebar {
            Some(sidebar) => {
                let title = sidebar.title.try_into_engine(&())?;
                let mut texts = vec![title.clone()];
                let mut lines = Vec::with_capacity(sidebar.lines.len().min(SIDEBAR_LINES));
                for line in sidebar.lines.iter().take(SIDEBAR_LINES) {
                    let text = line.text.try_into_engine(&())?;
                    let value: Option<horizon_client::text::Text> =
                        line.value.try_into_engine(&())?;
                    texts.extend([Some(text.clone()), value.clone()].into_iter().flatten());
                    lines.push((text, value));
                }
                // Checked before the old sidebar is taken away: a text the writer refuses would
                // leave the player with none.
                if texts.iter().any(|text| {
                    !horizon_core::nbt::within_depth(
                        &horizon_client::lan::data::component_nbt(&text.component()),
                        NBT_DEPTH,
                    )
                }) {
                    return Err(TextFormError::TooDeep);
                }
                Some(NativeSidebar { title, lines })
            }
            None => None,
        };
        Ok(self.native_sidebar(player.uuid(), sidebar.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_guest_turn_tests::with_guests;
    use horizon_client::lan::PlayerSender;
    use horizon_protocol::packets::play::{self, Clientbound as C};
    use mod_api::{Sidebar, SidebarLine, Text};
    use std::sync::{Arc, atomic::AtomicUsize};

    /// The sidebar reaches only that player: its objective, the sidebar slot and one score a
    /// line with the right-hand text as a fixed number format; None removes it.
    #[test]
    #[ignore = "daily"]
    fn player_set_sidebar_sends_the_objective_and_its_lines() {
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
            let a = game.player(12).unwrap();
            let sidebar = Sidebar::new(
                Text::literal("Stats"),
                vec![
                    SidebarLine::new(Text::literal("Kills"), Some(Text::literal("3"))),
                    SidebarLine::new(Text::literal("Coins"), None),
                ],
            );
            assert_eq!(game.player_set_sidebar(a, Some(sidebar)), Ok(true));
            let sent: Vec<C> = rx
                .try_iter()
                .filter_map(|out| match out {
                    horizon_client::lan::Out::Packet(body) => {
                        let mut r = horizon_protocol::buf::Reader::new(&body);
                        let id = r.varint().ok()?;
                        C::decode(id, &body[r.position()..]).ok()
                    }
                    _ => None,
                })
                .collect();
            let scores: Vec<_> = sent
                .iter()
                .filter_map(|p| match p {
                    C::SetScore(s) => Some((s.score, s.number_format.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(scores.len(), 2);
            assert_eq!(scores[0].0, 2, "top line scores highest");
            assert!(matches!(scores[0].1, Some(play::NumberFormat::Fixed(_))));
            assert!(matches!(scores[1].1, Some(play::NumberFormat::Blank)));
            assert!(
                sent.iter()
                    .any(|p| matches!(p, C::SetDisplayObjective(d) if d.slot == 1))
            );
            // clearing sends the removal of the objective the first call added
            rx.try_iter().for_each(drop);
            assert_eq!(game.player_set_sidebar(a, None), Ok(true));
            let cleared: Vec<C> = rx
                .try_iter()
                .filter_map(|out| match out {
                    horizon_client::lan::Out::Packet(body) => {
                        let mut r = horizon_protocol::buf::Reader::new(&body);
                        let id = r.varint().ok()?;
                        C::decode(id, &body[r.position()..]).ok()
                    }
                    _ => None,
                })
                .collect();
            assert!(
                cleared.iter().any(|p| matches!(
                    p,
                    C::SetObjective(o) if o.method == 1 && o.name == "horizon:native_sidebar"
                )),
                "{cleared:?}"
            );
            // 300 nested components pass the text depth check but their NBT is deeper than the writer
            // takes: refused for a player who is on (nothing sent), Ok(false) for one who is not
            let deep = (0..300).fold(Text::literal("x"), |inner, _| {
                Text::literal("").append(inner)
            });
            let refused = Sidebar::new(deep, vec![]);
            assert_eq!(
                game.player_set_sidebar(a, Some(refused.clone())),
                Err(mod_api::TextFormError::TooDeep)
            );
            assert!(rx.try_iter().next().is_none(), "nothing was sent");
            let gone = mod_api::__private::game::player_ref(a.entity(), 999);
            assert_eq!(game.player_set_sidebar(gone, Some(refused)), Ok(false));
        });
    }

    /// A player on a coloured team has the team's slot preferred over slot 1 by the client: the personal
    /// sidebar is shown in that slot too, and clearing gives both slots back to the world's objectives.
    #[test]
    #[ignore = "daily"]
    fn a_team_players_sidebar_takes_the_team_slot_and_clear_restores_the_worlds() {
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
            let a = game.player(12).unwrap();
            game.load_scoreboard();
            game.score.board.add_team("red").color = Some("red".to_owned());
            game.score.board.add_to_team("guest12", "red");
            assert!(game.score.board.add_objective(
                "world_team",
                "dummy",
                serde_json::json!("world"),
                horizon_client::scoreboard::RenderType::Integer,
                false,
                None,
            ));
            game.score
                .board
                .set_display_objective("sidebar.team.red", Some("world_team"));
            let slot = 3 + horizon_client::text::COLORS
                .iter()
                .position(|c| c.1 == "red")
                .unwrap() as i32;
            let displays = |rx: &std::sync::mpsc::Receiver<horizon_client::lan::Out>| {
                rx.try_iter()
                    .filter_map(|out| match out {
                        horizon_client::lan::Out::Packet(body) => {
                            let mut r = horizon_protocol::buf::Reader::new(&body);
                            let id = r.varint().ok()?;
                            match C::decode(id, &body[r.position()..]).ok()? {
                                C::SetDisplayObjective(d) => Some((d.slot, d.objective)),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let personal = Sidebar::new(Text::literal("Mine"), vec![]);
            assert_eq!(game.player_set_sidebar(a, Some(personal)), Ok(true));
            let shown = displays(&rx);
            assert!(
                shown.contains(&(1, "horizon:native_sidebar".to_owned())),
                "{shown:?}"
            );
            assert!(
                shown.contains(&(slot, "horizon:native_sidebar".to_owned())),
                "{shown:?}"
            );
            assert_eq!(game.player_set_sidebar(a, None), Ok(true));
            let restored = displays(&rx);
            assert_eq!(restored, [(slot, "world_team".to_owned())]);
        });
    }
}
