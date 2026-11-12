//! Host method declarations; access defaults and game forwarders are generated.
#![allow(unused_variables)]
use crate::*;
/// The methods of the `text` area on both sides, implemented by the game and reached through [`Host`].
#[horizon_mod_api_macros::host_area]
pub trait TextHost {
    /// Side: both.
    /// Reads a text component as `/tellraw` and `ComponentSerialization.CODEC` take it: one
    /// JSON or SNBT value (`"plain"`, `{"text":"Hi","color":"gold"}`, `["a",{"text":"b"}]`,
    /// `translate`, `extra`, click and hover events), with nothing but whitespace after it
    /// (Paper's `GsonComponentSerializer.deserialize`). None when it isn't one component.
    #[access(Metadata)]
    fn text_from_json(&self, json: &str) -> Result<Option<Text>, TextFormError>;
}
