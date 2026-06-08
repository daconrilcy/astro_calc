use astral_llm_domain::{
    TextLanguage, TextRetreatmentOperation, TextRetreatmentRequest, TextRetreatmentRequestContext,
    TextService, TextTarget,
};

#[test]
fn text_reprocessing_contracts_accept_open_language_and_service_codes() {
    let request = TextRetreatmentRequest {
        language: TextLanguage::new("test_lang"),
        service: TextService::new("test_service"),
        target: TextTarget::PlainText,
        operations: vec![TextRetreatmentOperation::Sanitize],
        payload: serde_json::json!("texte"),
        context: TextRetreatmentRequestContext::default(),
    };

    let json = serde_json::to_value(&request).expect("serialize");
    assert_eq!(json["language"]["code"], "test_lang");
    assert_eq!(json["service"]["code"], "test_service");
    assert_eq!(json["operations"][0], "sanitize");
}
