//! Tests for gathering, selection and the mod-backed handler. Expected values are literals.

use super::*;

/// A mod's answer as the test double gives it: the mod that answers and a non-capturing
/// function producing the answer. `None` answers nothing.
struct Reply {
    mod_id: &'static str,
    value: Option<fn() -> Result<PermissionValue, PermissionError>>,
}

impl ModAnswers for Reply {
    fn answer(
        &self,
        _handler: &str,
        _node: &PermissionNode,
        _online: Option<u128>,
        _uuid: u128,
        _contexts: &[PermissionContext],
    ) -> Option<Answered<Result<PermissionValue, PermissionError>>> {
        let make = self.value?;
        Some(Answered {
            answer: make(),
            mod_id: self.mod_id,
        })
    }
}

fn node(value_type: PermissionValueType) -> PermissionNode {
    PermissionNode {
        name: "garden.harvest_rare".to_owned(),
        value_type,
    }
}

fn handler(mod_id: &'static str, value: Option<fn() -> Result<PermissionValue, PermissionError>>) -> ModPermissionHandler {
    ModPermissionHandler {
        id: "garden.policy".to_owned(),
        owner: "garden".to_owned(),
        answers: Rc::new(Reply { mod_id, value }),
    }
}

fn gathered(ids: &[(&str, &'static str)]) -> Vec<Answered<String>> {
    ids.iter()
        .map(|(id, mod_id)| Answered {
            answer: (*id).to_owned(),
            mod_id: *mod_id,
        })
        .collect()
}

fn no_answers() -> Rc<dyn ModAnswers> {
    Rc::new(Reply {
        mod_id: "garden",
        value: None,
    })
}

#[test]
fn empty_setting_selects_no_handler() {
    let ids = gathered(&[("garden.policy", "garden")]);
    assert_eq!(configured_handler("", &ids), None);
}

#[test]
fn gathered_setting_selects_its_handler() {
    let ids = gathered(&[("garden.policy", "garden"), ("shop.policy", "shop")]);
    assert_eq!(
        configured_handler("shop.policy", &ids),
        Some("shop.policy".to_owned())
    );
}

#[test]
fn unknown_setting_selects_no_handler() {
    let ids = gathered(&[("garden.policy", "garden")]);
    assert_eq!(configured_handler("missing.policy", &ids), None);
}

#[test]
fn duplicate_handler_ids_fail_and_publish_nothing() {
    let mut service = PermissionService::new();
    let ids = gathered(&[("garden.policy", "garden"), ("garden.policy", "shop")]);
    let result = gather_handlers(&mut service, &ids, &no_answers(), "");
    assert!(matches!(result, Err(PermissionError::DuplicateHandler)));
    let query = service.query(&node(PermissionValueType::Boolean), None, 1, &[]);
    assert!(matches!(query, Err(PermissionError::NotInitialized)));
}

#[test]
fn empty_handler_id_is_invalid_input() {
    let mut service = PermissionService::new();
    let ids = gathered(&[("", "garden")]);
    let result = gather_handlers(&mut service, &ids, &no_answers(), "");
    assert!(matches!(result, Err(PermissionError::InvalidInput)));
}

#[test]
fn reopened_server_gathers_afresh() {
    let ids = gathered(&[("garden.policy", "garden")]);
    let mut first = PermissionService::new();
    assert!(gather_handlers(&mut first, &ids, &no_answers(), "garden.policy").is_ok());
    let mut second = PermissionService::new();
    assert!(gather_handlers(&mut second, &ids, &no_answers(), "garden.policy").is_ok());
}

#[test]
fn provider_answer_is_returned() {
    let mut handler = handler("garden", Some(|| Ok(PermissionValue::Boolean(true))));
    let answer = handler.query(&node(PermissionValueType::Boolean), Some(7), 7, &[]);
    assert!(matches!(answer, Ok(PermissionValue::Boolean(true))));
}

#[test]
fn no_answer_is_a_handler_failure() {
    let mut handler = handler("garden", None);
    let answer = handler.query(&node(PermissionValueType::Boolean), None, 7, &[]);
    assert!(matches!(answer, Err(PermissionError::HandlerFailed)));
}

#[test]
fn answer_from_another_mod_is_a_handler_failure() {
    let mut handler = handler("shop", Some(|| Ok(PermissionValue::Boolean(true))));
    let answer = handler.query(&node(PermissionValueType::Boolean), None, 7, &[]);
    assert!(matches!(answer, Err(PermissionError::HandlerFailed)));
}

#[test]
fn failed_answer_is_the_error_and_never_a_grant() {
    let mut handler = handler("garden", Some(|| Err(PermissionError::Recursive)));
    let answer = handler.query(&node(PermissionValueType::Boolean), None, 7, &[]);
    assert!(matches!(answer, Err(PermissionError::Recursive)));
}

#[test]
fn value_of_another_type_is_wrong_type() {
    let mut handler = handler("garden", Some(|| Ok(PermissionValue::Integer(2))));
    let answer = handler.query(&node(PermissionValueType::Boolean), None, 7, &[]);
    assert!(matches!(answer, Err(PermissionError::WrongType)));
}
