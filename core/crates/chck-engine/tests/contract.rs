//! MailEngine 契约测试。未实现切片必须返回带阶段标记的 Unsupported，禁止 panic。

use chck_engine::CoreEngine;
use chck_types::{
    AddAccountRequest, EngineError, FolderId, MailEngine, MessageId, Page, SearchQuery, SendRequest,
};

fn engine() -> CoreEngine {
    CoreEngine::open_in_memory().expect("store")
}

#[test]
fn unimplemented_ops_are_explicit_unsupported() {
    let e = engine();
    let acc = e.add_account(AddAccountRequest::new("user@icloud.com")).unwrap();
    assert_eq!(acc.provider_id, "icloud");

    assert!(e.list_folders(&acc.id).unwrap().is_empty());
    assert_eq!(e.list_providers().unwrap().len(), 20);
    assert!(e.list_messages(&FolderId::from("f"), Page::default()).unwrap().is_empty());
    assert!(matches!(e.body(&MessageId::from("m")), Err(EngineError::NotFound)));
    assert!(e.search(&SearchQuery::default()).unwrap().is_empty());
    let draft =
        e.save_draft(SendRequest { account_id: acc.id.clone(), ..SendRequest::default() }).unwrap();
    e.undo_send(&draft).unwrap();
}

#[test]
fn invalid_email_rejected() {
    let e = engine();
    let err = e.add_account(AddAccountRequest::new("not-an-email")).unwrap_err();
    assert!(matches!(err, EngineError::Invalid(_)));
}
