use chrono::{Duration, Utc};
use qsonaut_protocol::{ActivityVisibilityInput, DiagnosticReportInput, QsoLogInput, StationPresenceInput};
use qsonaut_store::{NewAccessRequest, Store};
use uuid::Uuid;

#[tokio::test]
async fn migrated_postgres_supports_challenges_visibility_and_log_retries() {
    let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
        eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping PostgreSQL contract test");
        return;
    };
    let store = Store::connect(&database_url)
        .await
        .expect("connect and migrate test database");
    let user_id = Uuid::new_v4();
    let callsign = format!("T{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
    sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Migration test','unused','member')")
        .bind(user_id)
        .bind(callsign)
        .execute(store.pool())
        .await
        .expect("insert test user");
    let other_user_id = Uuid::new_v4();
    let other_callsign = format!("T{}", &other_user_id.simple().to_string()[..8]).to_ascii_uppercase();
    sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Other migration test','unused','member')")
        .bind(other_user_id)
        .bind(other_callsign)
        .execute(store.pool())
        .await
        .expect("insert other test user");

    let diagnostic = DiagnosticReportInput {
        instance_id: Uuid::new_v4(),
        category: "hardware".to_owned(),
        summary: "operator validation".to_owned(),
        payload: serde_json::json!({"audio": "ok"}),
    };
    store
        .create_diagnostic_report(user_id, &diagnostic)
        .await
        .expect("create user diagnostic");
    store
        .create_diagnostic_report(other_user_id, &diagnostic)
        .await
        .expect("create other diagnostic");
    let own_diagnostics = store
        .diagnostic_reports_for_user(user_id, 100)
        .await
        .expect("load user diagnostics");
    assert_eq!(own_diagnostics.len(), 1);
    assert_eq!(own_diagnostics[0].user_id, user_id);

    let station = StationPresenceInput {
        instance_id: Uuid::new_v4(),
        station_label: "Contract station".to_owned(),
        radio_manufacturer: Some("Test".to_owned()),
        radio_model: Some("Radio".to_owned()),
        frequency_hz: Some(14_074_000),
        band: Some("20m".to_owned()),
        mode: Some("FT8".to_owned()),
        qsonaut_version: "test".to_owned(),
        platform: "linux".to_owned(),
        status: "online".to_owned(),
        metadata: serde_json::json!({}),
    };
    store
        .upsert_station_presence(user_id, &station)
        .await
        .expect("create user station");
    assert_eq!(store.station_presence(Some(user_id)).await.unwrap().len(), 1);
    assert_eq!(store.station_presence(Some(other_user_id)).await.unwrap().len(), 0);

    let challenge_id = Uuid::new_v4();
    store
        .create_access_challenge(challenge_id, 0, Utc::now() + Duration::minutes(10))
        .await
        .expect("create challenge");
    sqlx::query("UPDATE access_challenges SET attempts_remaining=1 WHERE id=$1")
        .bind(challenge_id)
        .execute(store.pool())
        .await
        .expect("prepare final challenge attempt");
    assert_eq!(
        store
            .take_access_challenge_attempt(challenge_id)
            .await
            .expect("take final challenge attempt"),
        Some((0, 0))
    );

    let private_policy = ActivityVisibilityInput {
        scope: "overall".to_owned(),
        scope_id: None,
        visibility: "private".to_owned(),
    };
    let first_policy = store
        .set_activity_visibility(user_id, &private_policy)
        .await
        .expect("insert visibility policy");
    let global_policy = ActivityVisibilityInput {
        visibility: "global".to_owned(),
        ..private_policy
    };
    let updated_policy = store
        .set_activity_visibility(user_id, &global_policy)
        .await
        .expect("update visibility policy");
    assert_eq!(updated_policy.id, first_policy.id);
    assert_eq!(updated_policy.visibility, "global");

    let log = QsoLogInput {
        event_id: None,
        operating_callsign: None,
        callsign_id: None,
        visibility: "private".to_owned(),
        visibility_club_id: None,
        idempotency_key: Uuid::new_v4(),
        callsign: "W1AW".to_owned(),
        band: "20m".to_owned(),
        mode: "FT8".to_owned(),
        frequency_hz: Some(14_074_000),
        occurred_at: Utc::now(),
        rst_sent: Some("-10".to_owned()),
        rst_received: Some("-12".to_owned()),
        exchange: serde_json::json!({}),
        points: 1,
        source: "contract-test".to_owned(),
    };
    let first_log = store
        .create_qso_log(user_id, &log)
        .await
        .expect("insert QSO log");
    let retried_log = store
        .create_qso_log(user_id, &log)
        .await
        .expect("retry QSO log");
    assert_eq!(retried_log.id, first_log.id);

    verify_access_approval(&store, user_id).await;
}

async fn verify_access_approval(store: &Store, reviewer_id: Uuid) {
    let request_id = Uuid::new_v4();
    let applicant_callsign =
        format!("A{}", &request_id.simple().to_string()[..8]).to_ascii_uppercase();
    let request = NewAccessRequest {
        id: request_id,
        callsign: &applicant_callsign,
        email: "applicant@example.test",
        club_name: "Review Club",
        referral_source: "PostgreSQL contract test",
        hamdb_display_name: "Test Applicant",
        hamdb_grid: "CN87",
        hamdb_license_class: "General",
        hamdb_license_status: "Active",
    };
    store
        .create_access_request(&request)
        .await
        .expect("create access request");
    let (approved, account) = store
        .approve_access_request(request_id, reviewer_id, "temporary-password-hash")
        .await
        .expect("approve access request")
        .expect("pending access request exists");
    assert_eq!(approved.status, "approved");
    assert_eq!(account.callsign, applicant_callsign);
    let profile = store
        .user_profile(account.id)
        .await
        .expect("load approved profile")
        .expect("approved profile exists");
    assert_eq!(profile.grid, "CN87");
    assert_eq!(profile.license_class, "General");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE action='access_request_approved' AND target_id=$1",
    )
    .bind(request_id)
    .fetch_one(store.pool())
    .await
    .expect("query approval audit event");
    assert_eq!(audit_count, 1);
}
