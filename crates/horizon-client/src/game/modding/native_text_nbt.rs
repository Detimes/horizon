//! `text_to_nbt`: a text as item components and block entities store it.
use super::Game;
use horizon_client::{lan::data::component_nbt, text::Text};
use horizon_core::nbt::{Tag, within_depth};
use mod_api::TextFormError;

/// The deepest NBT the writer takes.
const NBT_DEPTH: usize = 512;

/// A component's JSON as item components and block entities store it: a plain literal with no style
/// and no children is its string (as `ComponentSerialization` saves it, so two stacks that carry the
/// same plain name compare equal), anything else a compound. A form deeper than the NBT writer takes
/// is refused: each child adds an `extra` list and a compound, so a chain the text depth check let
/// through can still be too deep here.
fn component_to_nbt(component: &serde_json::Value) -> Result<Tag, TextFormError> {
    let tag = match component {
        serde_json::Value::Object(o)
            if o.len() == 1 && o.get("text").is_some_and(|t| t.is_string()) =>
        {
            component_nbt(&o["text"])
        }
        other => component_nbt(other),
    };
    if within_depth(&tag, NBT_DEPTH) {
        Ok(tag)
    } else {
        Err(TextFormError::TooDeep)
    }
}

impl Game<'_> {
    /// The text as item components store it.
    pub(super) fn text_to_nbt(&self, text: &Text) -> Result<Tag, TextFormError> {
        component_to_nbt(&text.component())
    }
}

#[cfg(test)]
mod tests {
    use super::{Text, component_nbt, component_to_nbt};
    use horizon_core::nbt::Tag;
    use mod_api::TextFormError;
    use serde_json::json;

    /// A plain unstyled literal is its string, a styled one stays a compound: both forms of the same
    /// plain name are one component.
    #[test]
    fn a_plain_text_is_its_string_and_a_styled_one_a_compound() {
        assert_eq!(
            component_to_nbt(&json!({"text": "Prize"})),
            Ok(Tag::String("Prize".into()))
        );
        assert!(matches!(
            component_to_nbt(&json!({"text": "Prize", "bold": true})),
            Ok(Tag::Compound(_))
        ));
        assert!(matches!(
            component_to_nbt(&json!({"text": "a", "extra": [{"text": "b"}]})),
            Ok(Tag::Compound(_))
        ));
    }

    /// A chain of 257 children passes the 512-node text check but its NBT has 513 nested containers.
    #[test]
    fn a_text_whose_nbt_is_too_deep_is_refused() {
        let chain = |n: usize| {
            (0..n).fold(
                json!({"text": "x"}),
                |inner, _| json!({"text": "", "extra": [inner]}),
            )
        };
        assert_eq!(component_to_nbt(&chain(257)), Err(TextFormError::TooDeep));
        assert!(component_to_nbt(&chain(200)).is_ok());
    }

    /// The NBT reads back as the same component, as an item's name does.
    #[test]
    fn text_to_nbt_is_the_components_nbt_form() {
        let text = Text::legacy("\u{a7}6Golden \u{a7}lsword");
        let tag = component_nbt(&text.component());
        let back = horizon_client::component::name_text(&tag);
        assert_eq!(back.plain(), "Golden sword");
        let styles: Vec<_> = back.chars().map(|(_, style)| style.clone()).collect();
        assert_eq!(styles[0].color, Some(0xFFAA00), "gold");
        assert!(!styles[0].is_bold());
        assert!(styles[7].is_bold(), "the second word is bold");
    }
}
