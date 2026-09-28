use super::*;

fn open() -> (tempfile::TempDir, CredentialStore, LocalDatabase) {
    let root = tempfile::tempdir_in("E:\\Codex\\temp").unwrap();
    let credentials = CredentialStore::test_fixture();
    let db = LocalDatabase::open(root.path().join("test.db"), &credentials).unwrap();
    (root, credentials, db)
}
fn available(db: &LocalDatabase) -> BasketItem {
    let (copy, title): (String, String) = db
        .conn
        .query_row(
            "SELECT id,book_title_id FROM book_copies WHERE status='AVAILABLE' LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    BasketItem {
        id: copy.clone(),
        copy_id: Some(copy),
        title_id: title,
        quantity: 1,
        mode: "COPY".into(),
        loan_id: None,
    }
}
#[test]
fn issued_loan_id_is_durable_idempotent_and_in_outbox_payload() {
    let (root, credentials, mut db) = open();
    let item = available(&db);
    let op = Uuid::new_v4().to_string();
    let result = db
        .issue("reader-ali", std::slice::from_ref(&item), &op)
        .unwrap();
    assert_eq!(
        result.loan_ids[0],
        crate::security::operation_entity_id(&op, "loan", 0)
    );
    assert_eq!(
        db.issue("reader-ali", &[item], &op).unwrap().loan_ids,
        result.loan_ids
    );
    assert_eq!(db.pending_outbox().unwrap().len(), 1);
    assert!(db.pending_outbox().unwrap()[0]
        .2
        .contains(&result.loan_ids[0]));
    drop(db);
    let reopened = LocalDatabase::open(root.path().join("test.db"), &credentials).unwrap();
    assert!(reopened
        .reader_loans("reader-ali")
        .unwrap()
        .iter()
        .any(|loan| loan.id == result.loan_ids[0]));
    assert_eq!(reopened.pending_outbox().unwrap().len(), 1);
}
#[test]
fn invalid_second_item_rolls_back_copy_and_outbox() {
    let (_root, _credentials, mut db) = open();
    let first = available(&db);
    let mut second = first.clone();
    second.copy_id = Some("nonexistent".into());
    second.id = "nonexistent".into();
    assert!(db
        .issue(
            "reader-ali",
            &[first.clone(), second],
            &Uuid::new_v4().to_string()
        )
        .is_err());
    assert_eq!(
        db.conn
            .query_row(
                "SELECT status FROM book_copies WHERE id=?1",
                params![first.copy_id],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "AVAILABLE"
    );
    assert!(db.pending_outbox().unwrap().is_empty());
}
#[test]
fn copy_title_quantity_and_accounting_mode_are_authoritative() {
    let (_root, _credentials, mut db) = open();
    let original = available(&db);
    for invalid in [
        BasketItem {
            title_id: "title-russian".into(),
            ..original.clone()
        },
        BasketItem {
            quantity: 2,
            ..original.clone()
        },
        BasketItem {
            mode: "bogus".into(),
            ..original.clone()
        },
    ] {
        assert!(db
            .issue("reader-ali", &[invalid], &Uuid::new_v4().to_string())
            .is_err());
    }
    assert!(db.pending_outbox().unwrap().is_empty());
}
#[test]
fn partial_return_foreign_reader_and_wrong_title_do_not_corrupt_stock() {
    let (_root, _credentials, mut db) = open();
    let item = BasketItem {
        id: "loan-russian".into(),
        title_id: "title-russian".into(),
        copy_id: None,
        quantity: 1,
        mode: "LEGACY_TITLE".into(),
        loan_id: Some("loan-russian".into()),
    };
    assert!(db
        .accept(
            "reader-dana",
            std::slice::from_ref(&item),
            &Uuid::new_v4().to_string()
        )
        .is_err());
    assert!(db
        .accept(
            "reader-ali",
            &[BasketItem {
                title_id: "title-math".into(),
                ..item.clone()
            }],
            &Uuid::new_v4().to_string()
        )
        .is_err());
    assert!(db
        .accept(
            "reader-ali",
            &[BasketItem {
                quantity: 0,
                ..item.clone()
            }],
            &Uuid::new_v4().to_string()
        )
        .is_err());
    let op = Uuid::new_v4().to_string();
    db.accept("reader-ali", std::slice::from_ref(&item), &op)
        .unwrap();
    db.accept("reader-ali", &[item], &op).unwrap();
    assert_eq!(
        db.reader_loans("reader-ali")
            .unwrap()
            .iter()
            .find(|l| l.id == "loan-russian")
            .unwrap()
            .quantity,
        1
    );
    assert_eq!(
        db.book_availability("title-russian")
            .unwrap()
            .available_legacy_quantity,
        3
    );
    assert_eq!(db.snapshot(false).unwrap().legacy_stock["title-russian"], 4);
    assert_eq!(db.pending_outbox().unwrap().len(), 1);
}
#[test]
fn offline_issue_return_reservation_persist_as_three_atomic_operations() {
    let (_root, _credentials, mut db) = open();
    let item = available(&db);
    let issued = db
        .issue(
            "reader-ali",
            std::slice::from_ref(&item),
            &Uuid::new_v4().to_string(),
        )
        .unwrap();
    let returned = BasketItem {
        loan_id: Some(issued.loan_ids[0].clone()),
        ..item.clone()
    };
    db.accept("reader-ali", &[returned], &Uuid::new_v4().to_string())
        .unwrap();
    let op = Uuid::new_v4().to_string();
    let reserve = db.reserve("reader-ali", &item.title_id, &op).unwrap();
    assert_eq!(
        reserve.reservation_id,
        Some(crate::security::operation_entity_id(&op, "reservation", 0))
    );
    assert!(db
        .reserve("reader-ali", &item.title_id, &Uuid::new_v4().to_string())
        .is_err());
    assert_eq!(db.pending_outbox().unwrap().len(), 3);
    assert_eq!(
        db.conn
            .query_row(
                "SELECT status FROM book_copies WHERE id=?1",
                params![item.copy_id],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "AVAILABLE"
    );
}
#[test]
fn scanner_binding_uses_all_codes_without_changing_inventory_string() {
    let (_root, _credentials, mut db) = open();
    let item = available(&db);
    let id = item.copy_id.unwrap();
    db.bind_test_book_code(&id, "000A-009").unwrap();
    assert_eq!(db.resolve_code("000A-009").unwrap().copy.unwrap().id, id);
    assert!(db.resolve_code("unknown-code").unwrap().copy.is_none());
}
#[test]
fn reset_with_committed_operations_is_atomic_and_preserves_production_namespace() {
    let (_root, _credentials, mut db) = open();
    let reader = db
        .create_test_reader(
            &test_input(
                "UAT-RESET-OP",
                "Тест",
                "STUDENT",
                Some("7 А"),
                None,
                "ACTIVE",
            ),
            false,
        )
        .unwrap()
        .reader;
    db.issue(&reader.id, &[available(&db)], &Uuid::new_v4().to_string())
        .unwrap();
    db.reset_test_data().unwrap();
    assert!(db.readers("").unwrap().is_empty());
    assert!(db.copies().unwrap().is_empty());
    assert!(db.pending_outbox().unwrap().is_empty());
    assert_eq!(
        db.conn
            .query_row("SELECT COUNT(*) FROM library_operations", [], |r| r
                .get::<_, i32>(0))
            .unwrap(),
        0
    );
}
#[test]
fn production_has_no_seed_and_rejects_test_mutations() {
    let root = tempfile::tempdir_in("E:\\Codex\\temp").unwrap();
    let credentials = CredentialStore::test_production_fixture();
    let mut db = LocalDatabase::open(root.path().join("production.db"), &credentials).unwrap();
    assert!(db.snapshot(false).unwrap().readers.is_empty());
    assert!(db.snapshot(false).unwrap().titles.is_empty());
    assert_eq!(db.reset_test_data().unwrap_err().code, "TEST_MODE_REQUIRED");
    assert_eq!(
        db.create_test_reader(
            &test_input("X", "Тест", "STAFF", None, None, "ACTIVE"),
            false
        )
        .unwrap_err()
        .code,
        "TEST_MODE_REQUIRED"
    );
}
#[test]
fn canonical_partial_return_preserves_remaining_and_rejects_invalid_counts() {
    let value = serde_json::json!({"quantity":2,"returned_quantity":1,"status":"ACTIVE"});
    assert_eq!(
        projection_remaining_quantity(value.as_object().unwrap()).unwrap(),
        1
    );
    let invalid = serde_json::json!({"quantity":2,"returned_quantity":3,"status":"ACTIVE"});
    assert!(projection_remaining_quantity(invalid.as_object().unwrap()).is_err());
}

#[test]
fn backup_uses_sqlcipher_and_preserves_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = CredentialStore::test_fixture();
    let mut db = LocalDatabase::open(dir.path().join("backup-source.db"), &credentials).unwrap();
    let item = BasketItem {
        id: "copy-clean".into(),
        title_id: "title-clean".into(),
        copy_id: Some("copy-clean".into()),
        quantity: 1,
        mode: "COPY".into(),
        loan_id: None,
    };
    // Resolve an actually available fixture instead of assuming fixture IDs.
    let snapshot = db.snapshot(false).unwrap();
    let copy = snapshot
        .copies
        .iter()
        .find(|c| c.status == "AVAILABLE")
        .unwrap();
    let item = BasketItem {
        title_id: copy.title_id.clone(),
        copy_id: Some(copy.id.clone()),
        ..item
    };
    let receipt = db
        .issue("reader-ayla", &[item], &Uuid::new_v4().to_string())
        .unwrap();
    let path = db
        .encrypted_backup(&credentials)
        .expect("encrypted backup API");
    let unkeyed = Connection::open(&path).unwrap();
    assert!(unkeyed
        .query_row("SELECT count(*) FROM persons", [], |r| r.get::<_, i64>(0))
        .is_err());
    drop(unkeyed);
    let restored = LocalDatabase::open(path, &credentials).unwrap();
    assert_eq!(restored.outbox_counts().unwrap().0, 1);
    assert_eq!(
        restored
            .operation_result(&receipt.operation_id)
            .unwrap()
            .unwrap()
            .loan_ids,
        receipt.loan_ids
    );
    assert_eq!(restored.health_check().unwrap(), "ok");
}

#[test]
fn fts_search_handles_cyrillic_case_and_isbn_without_sql_injection() {
    let (_root, _credentials, db) = open();
    assert!(db
        .titles("математ")
        .unwrap()
        .iter()
        .any(|t| t.id == "title-math"));
    assert!(db
        .titles("АБЫЛКАСЫМОВА")
        .unwrap()
        .iter()
        .any(|t| t.id == "title-math"));
    assert!(db
        .titles("978-601-0123456")
        .unwrap()
        .iter()
        .any(|t| t.id == "title-math"));
    let before = db.titles("").unwrap().len();
    let _ = db.titles("'; DROP TABLE book_titles; --").unwrap();
    assert_eq!(db.titles("").unwrap().len(), before);
}
#[test]
fn repeated_legacy_title_in_issue_draft_is_rejected_atomically() {
    let (_root, _credentials, mut db) = open();
    let a = BasketItem {
        id: "a".into(),
        title_id: "title-russian".into(),
        copy_id: None,
        loan_id: Some("a".into()),
        mode: "LEGACY_TITLE".into(),
        quantity: 1,
    };
    let mut b = a.clone();
    b.id = "b".into();
    b.loan_id = Some("b".into());
    assert!(db
        .issue("reader-ali", &[a, b], &Uuid::new_v4().to_string())
        .is_err());
    assert!(db.pending_outbox().unwrap().is_empty());
}

#[test]
fn sync_status_remains_readable_without_transport_mutex_and_reports_offline_queue() {
    let (_root, _credentials, mut db) = open();
    let engine = std::sync::Mutex::new(crate::sync::SyncEngine::with_production(None));
    let _transport_busy = engine.lock().unwrap();
    db.issue(
        "reader-ayla",
        &[available(&db)],
        &Uuid::new_v4().to_string(),
    )
    .unwrap();
    let status = crate::sync::SyncEngine::status_for(&db, true, false).unwrap();
    assert!(status.local_ready);
    assert!(!status.online);
    assert_eq!(status.state, "OFFLINE_QUEUE");
    assert_eq!(status.pending_count, 1);
}

#[test]
fn uat_restore_is_encrypted_atomic_and_keeps_current_recovery_snapshot() {
    let (_root, credentials, mut db) = open();
    db.conn.execute("INSERT INTO app_settings(key,value,updated_at) VALUES('restore-proof','before','2026-09-25')", []).unwrap();
    let target = db.encrypted_backup(&credentials).unwrap();
    let name = target.file_name().unwrap().to_str().unwrap();
    db.conn
        .execute(
            "UPDATE app_settings SET value='after' WHERE key='restore-proof'",
            [],
        )
        .unwrap();
    let report = db.restore_backup(name, &credentials).unwrap();
    assert_eq!(
        db.conn
            .query_row(
                "SELECT value FROM app_settings WHERE key='restore-proof'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "before"
    );
    let recovery = target.parent().unwrap().join(report.recovery_file_name);
    let snapshot =
        Connection::open_with_flags(recovery, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    snapshot
        .pragma_update(None, "key", credentials.existing_database_key().unwrap())
        .unwrap();
    assert_eq!(
        snapshot
            .query_row(
                "SELECT value FROM app_settings WHERE key='restore-proof'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "after"
    );
    let without_key = Connection::open(&target).unwrap();
    assert!(without_key
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| r
            .get::<_, i64>(0))
        .is_err());
    assert!(target.exists());
    assert_eq!(db.list_backups().unwrap().len(), 2);
    let path = db.path.clone();
    drop(db);
    assert_eq!(
        LocalDatabase::open(path, &credentials)
            .unwrap()
            .health_check()
            .unwrap(),
        "ok"
    );
}
#[test]
fn restore_rejects_arbitrary_paths_corrupt_and_foreign_key_invalid_backup() {
    let (_root, credentials, mut db) = open();
    for name in [
        "../library.db",
        "C:\\Windows\\secret",
        "library-x.db",
        "library.db",
    ] {
        assert_eq!(
            db.restore_backup(name, &credentials).unwrap_err().code,
            "BACKUP_NAME_INVALID"
        );
    }
    let target = db.encrypted_backup(&credentials).unwrap();
    let name = target.file_name().unwrap().to_str().unwrap();
    let invalid = Connection::open(&target).unwrap();
    invalid
        .pragma_update(None, "key", credentials.existing_database_key().unwrap())
        .unwrap();
    invalid
        .execute_batch("PRAGMA foreign_keys=OFF; UPDATE terminals SET school_id='missing-school';")
        .unwrap();
    drop(invalid);
    assert_eq!(
        db.restore_backup(name, &credentials).unwrap_err().code,
        "BACKUP_INTEGRITY_FAILED"
    );
    assert_eq!(db.health_check().unwrap(), "ok");
    fs::write(&target, vec![0u8; 8192]).unwrap();
    assert!(db.restore_backup(name, &credentials).is_err());
    assert_eq!(db.health_check().unwrap(), "ok");
}
#[test]
fn production_restore_never_rewinds_cloud_state() {
    let root = tempfile::tempdir().unwrap();
    let credentials = CredentialStore::named_test_production_fixture("production-restore-denied");
    let mut db = LocalDatabase::open(root.path().join("production.db"), &credentials).unwrap();
    assert_eq!(
        db.restore_backup(
            "library-00000000-0000-0000-0000-000000000000.db",
            &credentials
        )
        .unwrap_err()
        .code,
        "TEST_MODE_REQUIRED"
    );
}
