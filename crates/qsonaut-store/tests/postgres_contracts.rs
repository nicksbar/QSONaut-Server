use chrono::{Duration, Utc};
use qsonaut_protocol::{
    ActivityVisibilityInput, ClubInput, ClubMembershipInput, DiagnosticReportInput, QsoLogInput,
    StationPresenceInput,
};
use qsonaut_store::{NewAccessRequest, Store};
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)]
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
        .bind(&callsign)
        .execute(store.pool())
        .await
        .expect("insert test user");
    let other_user_id = Uuid::new_v4();
    let other_callsign =
        format!("T{}", &other_user_id.simple().to_string()[..8]).to_ascii_uppercase();
    sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Other migration test','unused','member')")
        .bind(other_user_id)
        .bind(&other_callsign)
        .execute(store.pool())
        .await
        .expect("insert other test user");
    let personal_identity_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM managed_callsigns WHERE owner_user_id=$1 AND identity_type='personal'",
    )
    .bind(user_id)
    .fetch_one(store.pool())
    .await
    .expect("provisioned personal callsign");
    let other_identity_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM managed_callsigns WHERE owner_user_id=$1 AND identity_type='personal'",
    )
    .bind(other_user_id)
    .fetch_one(store.pool())
    .await
    .expect("provisioned other personal callsign");

    let club_suffix = &user_id.simple().to_string()[..8];
    let club = store
        .create_club(
            &ClubInput {
                name: format!("Contract Club {club_suffix}"),
                callsign: Some(
                    format!("N{}", &user_id.simple().to_string()[..6]).to_ascii_uppercase(),
                ),
                description: "membership contract".to_owned(),
            },
            user_id,
        )
        .await
        .expect("create contract club");
    let request = store
        .request_club_join(club.id, other_user_id)
        .await
        .expect("request club membership");
    assert_eq!(request.status, "pending");
    let approved = store
        .review_club_join_request(club.id, request.id, user_id, "approved", "operator")
        .await
        .expect("review club membership")
        .expect("pending membership request");
    assert_eq!(approved.status, "approved");
    assert!(
        store
            .active_club_member(other_user_id, club.id)
            .await
            .unwrap()
    );
    let membership = store
        .set_club_member(
            club.id,
            other_user_id,
            &ClubMembershipInput {
                user_id: other_user_id,
                role: "operator".to_owned(),
                membership_status: Some("lapsed".to_owned()),
                dues_status: Some("overdue".to_owned()),
                membership_number: Some("42".to_owned()),
                renewal_due_on: None,
            },
        )
        .await
        .expect("lapse club membership");
    assert_eq!(membership.membership_status, "lapsed");
    assert!(
        !store
            .active_club_member(other_user_id, club.id)
            .await
            .unwrap()
    );
    let other_clubs = store
        .clubs(other_user_id, false)
        .await
        .expect("load member organizations");
    let other_club = other_clubs
        .iter()
        .find(|item| item.id == club.id)
        .expect("club remains discoverable");
    assert_eq!(other_club.my_role.as_deref(), Some("operator"));
    assert_eq!(other_club.my_membership_status.as_deref(), Some("lapsed"));
    let owner_clubs = store
        .clubs(user_id, false)
        .await
        .expect("load owner organizations");
    let owner_club = owner_clubs
        .iter()
        .find(|item| item.id == club.id)
        .expect("owner club exists");
    assert_eq!(owner_club.my_role.as_deref(), Some("owner"));
    assert_eq!(owner_club.my_membership_status.as_deref(), Some("active"));

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
    assert_eq!(
        store.station_presence(Some(user_id)).await.unwrap().len(),
        1
    );
    assert_eq!(
        store
            .station_presence(Some(other_user_id))
            .await
            .unwrap()
            .len(),
        0
    );

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
        scope: "identity".to_owned(),
        scope_id: personal_identity_id,
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
        operating_callsign: Some(callsign.clone()),
        callsign_id: Some(personal_identity_id),
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
    let other_log = QsoLogInput {
        idempotency_key: Uuid::new_v4(),
        callsign: "K1ABC".to_owned(),
        operating_callsign: Some(other_callsign.clone()),
        callsign_id: Some(other_identity_id),
        ..log.clone()
    };
    store
        .create_qso_log(other_user_id, &other_log)
        .await
        .expect("insert other private log");
    store
        .set_activity_visibility(
            other_user_id,
            &ActivityVisibilityInput {
                scope: "identity".to_owned(),
                scope_id: other_identity_id,
                visibility: "global".to_owned(),
            },
        )
        .await
        .expect("make other log globally visible");
    let visible_logs = store
        .qso_logs_for_viewer(user_id, false, 100)
        .await
        .expect("load viewer logs");
    assert!(visible_logs.iter().any(|item| item.user_id == user_id));
    assert!(
        visible_logs
            .iter()
            .any(|item| item.user_id == other_user_id)
    );
    let summary = store
        .activity_summary(user_id, "overall", None, 0)
        .await
        .expect("summarize user activity");
    assert_eq!(summary.qso_count, 1);
    assert_eq!(summary.points, 1);
    assert_eq!(summary.status, "active");

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
