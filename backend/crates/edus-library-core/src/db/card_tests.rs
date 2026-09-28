use super::*;
use crate::identity::IdentityService;
fn person() -> TestReaderInput {
    TestReaderInput {
        external_id: "UAT-CARD-RESTART".into(),
        full_name: "Тестовый читатель Карта".into(),
        person_type: "STUDENT".into(),
        class_name: Some("Тестовый класс 7".into()),
        position_name: None,
        status: "ACTIVE".into(),
    }
}
#[test]
fn card_bind_restart_identity_uses_persisted_credential_and_namespace() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("uat/library.db");
    let credentials = CredentialStore::test_fixture();
    let mut db = LocalDatabase::open(path.clone(), &credentials).unwrap();
    let reader = db.create_test_reader(&person(), false).unwrap().reader;
    let secret = credentials.card_hmac_secret().unwrap();
    db.bind_test_reader_card(&reader.id, "00:ab-cd:0012", &secret)
        .unwrap();
    let before: i64 = db
        .conn
        .query_row("SELECT count(*) FROM sync_outbox", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, 0);
    drop(db);
    let reopened_credentials = CredentialStore::test_fixture();
    assert!(secret == reopened_credentials.card_hmac_secret().unwrap());
    let db = LocalDatabase::open(path, &reopened_credentials).unwrap();
    let identity = IdentityService::new(reopened_credentials);
    for raw in ["00:ab-cd:0012", "00ABCD0012\r\n", "00ab cd0012\t"] {
        assert_eq!(identity.card(&db, raw).unwrap().person_id, reader.id);
    }
    assert!(matches!(identity.card(&db,"00ABCD0013"),Err(e) if e.code=="NOT_FOUND"));
    let production_credentials =
        CredentialStore::named_test_production_fixture("card-isolation-production");
    production_credentials
        .save_card_hmac_secret("different-production-fixture-secret-0001")
        .unwrap();
    let production = LocalDatabase::open(
        directory.path().join("production/library.db"),
        &production_credentials,
    )
    .unwrap();
    assert!(
        matches!(IdentityService::new(production_credentials).card(&production,"00ABCD0012"),Err(e) if e.code=="NOT_FOUND")
    );
    let stored: String = db
        .conn
        .query_row(
            "SELECT card_lookup_hash FROM cards WHERE person_id=?1",
            params![reader.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored.len(), 64);
    assert!(!stored.contains("00ABCD0012"));
}
#[test]
#[ignore = "Explicit isolated native UAT fixture on E; no administrator session bypass is compiled into the app"]
fn prepare_native_card_restart_fixture() {
    let root = PathBuf::from(std::env::var("EDUS_CARD_QA_ROOT").expect("explicit E QA path"));
    assert!(root.starts_with(r"E:\Codex\temp"));
    assert!(
        !root.join("uat/library.db").exists(),
        "Never overwrite an existing workspace"
    );
    let credentials = CredentialStore::for_service_workspace(&root, true);
    let mut db = LocalDatabase::open(root.join("uat/library.db"), &credentials).unwrap();
    let reader = db.create_test_reader(&person(), false).unwrap().reader;
    db.bind_test_reader_card(
        &reader.id,
        "00:ab-cd:0012",
        &credentials.card_hmac_secret().unwrap(),
    )
    .unwrap();
    assert_eq!(
        IdentityService::new(credentials)
            .card(&db, "00ABCD0012")
            .unwrap()
            .person_id,
        reader.id
    );
    fs::write(root.join("workspace.json"),serde_json::to_vec(&serde_json::json!({"mode":"UAT","cloudUrl":null,"terminalId":null,"schoolId":null,"deviceName":"Card restart QA"})).unwrap()).unwrap();
    fs::write(root.join("qa-reader-id.txt"), reader.id).unwrap();
}

#[test]
fn diagnostics_bind_commits_rereads_and_never_exposes_raw_or_secret() {
    let directory = tempfile::tempdir().unwrap();
    let credentials = CredentialStore::test_fixture();
    let mut db = LocalDatabase::open(directory.path().join("uat.db"), &credentials).unwrap();
    let reader = db.create_test_reader(&person(), false).unwrap().reader;
    let service = crate::card::CardDiagnostics::new(credentials.clone());
    let raw = "0000AABBCC\t";
    let report = service.diagnose(&db, raw).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("0000AABBCC"));
    assert!(!json.contains(&credentials.card_hmac_secret().unwrap()));
    assert!(json.contains("Tab"));
    assert!(json.contains("kz.edus.library.automated-tests"));
    assert!(!report.hash_found);
    let token = report.capture_token.unwrap();
    assert_eq!(
        service.bind(&mut db, &reader.id, &token).unwrap().id,
        reader.id
    );
    assert!(service.bind(&mut db, &reader.id, &token).is_err());
    let resolved = service.diagnose(&db, raw).unwrap();
    assert!(resolved.hash_found && resolved.reader_resolved);
    let mut other = person();
    other.external_id = "UAT-CARD-OTHER".into();
    let other = db.create_test_reader(&other, false).unwrap().reader;
    assert!(
        matches!(db.bind_test_reader_card(&other.id,raw,&credentials.card_hmac_secret().unwrap()),Err(e) if e.code=="CARD_ALREADY_BOUND")
    );
    // Revoke by replacing this reader's active card, then restore their original card.
    db.bind_test_reader_card(
        &reader.id,
        "0000DDDD",
        &credentials.card_hmac_secret().unwrap(),
    )
    .unwrap();
    db.bind_test_reader_card(&reader.id, raw, &credentials.card_hmac_secret().unwrap())
        .unwrap();
    assert_eq!(
        IdentityService::new(credentials)
            .card(&db, raw)
            .unwrap()
            .person_id,
        reader.id
    );
}
#[test]
fn inactive_reader_cannot_receive_a_false_successful_card_binding() {
    let directory = tempfile::tempdir().unwrap();
    let credentials = CredentialStore::test_fixture();
    let mut db = LocalDatabase::open(directory.path().join("uat.db"), &credentials).unwrap();
    let mut input = person();
    input.status = "INACTIVE".into();
    let reader = db.create_test_reader(&input, false).unwrap().reader;
    assert!(db
        .bind_test_reader_card(
            &reader.id,
            "INACTIVE-CARD",
            &credentials.card_hmac_secret().unwrap()
        )
        .is_err());
    assert!(!db
        .card_hash_exists(
            &card_lookup_hash("INACTIVE-CARD", &credentials.card_hmac_secret().unwrap()).unwrap()
        )
        .unwrap());
}
#[test]
fn production_diagnostics_never_offer_card_binding() {
    let directory = tempfile::tempdir().unwrap();
    let credentials = CredentialStore::test_production_fixture();
    credentials
        .save_card_hmac_secret("fixture-production-secret-at-least-32")
        .unwrap();
    let mut db = LocalDatabase::open(directory.path().join("production.db"), &credentials).unwrap();
    let service = crate::card::CardDiagnostics::new(credentials);
    let report = service.diagnose(&db, "00001234").unwrap();
    assert!(report.capture_token.is_none());
    assert!(
        matches!(service.bind(&mut db,"reader","token"),Err(e) if e.code=="TEST_MODE_REQUIRED")
    );
}
