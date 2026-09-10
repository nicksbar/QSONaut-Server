use chrono::{Duration, Utc};
use qsonaut_protocol::{ManagedCallsignInput, QsoLogInput};
use qsonaut_store::Store;
use uuid::Uuid;

#[tokio::test]
async fn event_logs_require_active_operator_and_in_window_time() {
    let Ok(url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
        eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping event authorization contract");
        return;
    };
    let store = Store::connect(&url).await.unwrap();
    let user = Uuid::new_v4();
    let club = Uuid::new_v4();
    let event = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query("INSERT INTO users(id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Contest test','unused','member')")
        .bind(user).bind(format!("T{}", user.simple()).to_uppercase()).execute(store.pool()).await.unwrap();
    sqlx::query("INSERT INTO clubs(id,name) VALUES ($1,$2)")
        .bind(club)
        .bind(format!("Contest {club}"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO events(id,club_id,name,starts_at,ends_at,status) VALUES ($1,$2,'Test event',$3,$4,'active')")
        .bind(event).bind(club).bind(now - Duration::hours(1)).bind(now + Duration::hours(1)).execute(store.pool()).await.unwrap();
    let mut input = QsoLogInput {
        event_id: Some(event),
        operating_callsign: None,
        callsign_id: None,
        visibility: "private".into(),
        visibility_club_id: None,
        idempotency_key: Uuid::new_v4(),
        callsign: "W1AW".into(),
        band: "20m".into(),
        mode: "CW".into(),
        frequency_hz: Some(14_050_000),
        occurred_at: now,
        rst_sent: None,
        rst_received: None,
        exchange: serde_json::json!({}),
        points: 0,
        source: "contract-test".into(),
    };
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("INSERT INTO club_members(club_id,user_id,role) VALUES ($1,$2,'observer')")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("UPDATE club_members SET role='operator',membership_status='lapsed' WHERE club_id=$1 AND user_id=$2")
        .bind(club).bind(user).execute(store.pool()).await.unwrap();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query(
        "UPDATE club_members SET membership_status='active' WHERE club_id=$1 AND user_id=$2",
    )
    .bind(club)
    .bind(user)
    .execute(store.pool())
    .await
    .unwrap();
    let callsign_id = Uuid::new_v4();
    let operating_callsign = format!("K{}", &callsign_id.simple().to_string()[..6]).to_uppercase();
    sqlx::query("INSERT INTO managed_callsigns(id,callsign,identity_type,club_id,status,verification_status,authority,effective_from) VALUES ($1,$2,'club',$3,'active','verified','contract-test',$4)")
        .bind(callsign_id)
        .bind(&operating_callsign)
        .bind(club)
        .bind(now - Duration::hours(2))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO event_participants(id,event_id,user_id,club_id,callsign_id,operator_callsign,operating_callsign,role,status,starts_at,ends_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'operator','active',$8,$9)")
        .bind(Uuid::new_v4()).bind(event).bind(user).bind(club).bind(callsign_id)
        .bind(format!("T{}", user.simple()).to_uppercase()).bind(&operating_callsign)
        .bind(now - Duration::hours(1)).bind(now + Duration::hours(1)).execute(store.pool()).await.unwrap();
    input.operating_callsign = Some(operating_callsign.clone());
    input.callsign_id = Some(callsign_id);
    input.occurred_at = now + Duration::hours(1);
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.occurred_at = now - Duration::hours(2);
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.occurred_at = now;
    let accepted = store.create_qso_log(user, &input).await.unwrap();
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(event)
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        store.create_qso_log(user, &input).await.unwrap().id,
        accepted.id
    );
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("UPDATE events SET status='completed' WHERE id=$1")
        .bind(event)
        .execute(store.pool())
        .await
        .unwrap();
    assert!(store.create_qso_log(user, &input).await.is_ok());
}

#[tokio::test]
async fn contest_definition_validates_exchange_scores_and_marks_duplicates() {
    let Ok(url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
        eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping contest adjudication contract");
        return;
    };
    let store = Store::connect(&url).await.unwrap();
    let user = Uuid::new_v4();
    let club = Uuid::new_v4();
    let event = Uuid::new_v4();
    let callsign_id = Uuid::new_v4();
    let operating_callsign = format!("K{}", &callsign_id.simple().to_string()[..6]).to_uppercase();
    let now = Utc::now();
    let template_id = Uuid::parse_str("10000000-0000-4000-8000-000000000001").unwrap();
    sqlx::query("INSERT INTO users(id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Adjudication test','unused','member')")
        .bind(user).bind(format!("U{}", user.simple()).to_uppercase()).execute(store.pool()).await.unwrap();
    sqlx::query("INSERT INTO clubs(id,name) VALUES ($1,$2)")
        .bind(club)
        .bind(format!("Adjudication {club}"))
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO events(id,club_id,name,starts_at,ends_at,status,contest_template_id,contest_config) VALUES ($1,$2,'Field Day test',$3,$4,'active',$5,$6)")
        .bind(event).bind(club).bind(now - Duration::hours(1)).bind(now + Duration::hours(1))
        .bind(template_id).bind(serde_json::json!({"class":"1A","section":"WMA","power":"LOW"}))
        .execute(store.pool()).await.unwrap();
    let invalid_event = sqlx::query("INSERT INTO events(id,club_id,name,starts_at,ends_at,status,contest_template_id,contest_config) VALUES ($1,$2,'Invalid Field Day test',$3,$4,'active',$5,$6)")
        .bind(Uuid::new_v4()).bind(club).bind(now - Duration::hours(1)).bind(now + Duration::hours(1))
        .bind(template_id).bind(serde_json::json!({"class":"9Z","section":"WMA","power":"LOW"}))
        .execute(store.pool()).await;
    assert!(
        matches!(invalid_event, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("P1001"))
    );
    sqlx::query("INSERT INTO club_members(club_id,user_id,role) VALUES ($1,$2,'operator')")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    assert_special_callsign_audit(&store, event, user).await;
    sqlx::query("INSERT INTO managed_callsigns(id,callsign,identity_type,club_id,status,verification_status,authority,effective_from) VALUES ($1,$2,'club',$3,'active','verified','contract-test',$4)")
        .bind(callsign_id).bind(&operating_callsign).bind(club).bind(now - Duration::hours(2))
        .execute(store.pool()).await.unwrap();
    sqlx::query("INSERT INTO event_participants(id,event_id,user_id,club_id,callsign_id,operator_callsign,operating_callsign,role,status,starts_at,ends_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'operator','active',$8,$9)")
        .bind(Uuid::new_v4()).bind(event).bind(user).bind(club).bind(callsign_id)
        .bind(format!("U{}", user.simple()).to_uppercase()).bind(&operating_callsign)
        .bind(now - Duration::hours(1)).bind(now + Duration::hours(1))
        .execute(store.pool()).await.unwrap();
    let mut input = QsoLogInput {
        event_id: Some(event),
        operating_callsign: Some(operating_callsign),
        callsign_id: Some(callsign_id),
        visibility: "private".into(),
        visibility_club_id: None,
        idempotency_key: Uuid::new_v4(),
        callsign: "W1AW".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        frequency_hz: Some(14_074_000),
        occurred_at: now,
        rst_sent: None,
        rst_received: None,
        exchange: serde_json::json!({
            "fields_sent": {"class":"1A","section":"WMA"},
            "fields_received": {"class":"2A","section":"EMA"}
        }),
        points: 999,
        source: "contract-test".into(),
    };
    let accepted = store.create_qso_log(user, &input).await.unwrap();
    assert_eq!(accepted.points, 1);
    assert!(!accepted.is_duplicate);
    assert_eq!(accepted.scoring_version.as_deref(), Some("ARRL_FD:1"));
    assert_eq!(accepted.multipliers, serde_json::json!({"section": "EMA"}));
    assert!(accepted.scoring_explanation.contains("multiplier earned"));
    input.idempotency_key = Uuid::new_v4();
    let duplicate = store.create_qso_log(user, &input).await.unwrap();
    assert!(duplicate.is_duplicate);
    assert_eq!(duplicate.points, 0);
    let event_score = store.event_score(event).await.unwrap();
    assert_eq!(event_score.total_points, 1);
    assert_eq!(event_score.qso_count, 2);
    assert_eq!(event_score.duplicate_count, 1);
    assert_eq!(
        event_score.multiplier_values,
        serde_json::json!([{"section":"EMA"}])
    );
    sqlx::query("UPDATE events SET contest_definition_version=99 WHERE id=$1")
        .bind(event)
        .execute(store.pool())
        .await
        .unwrap();
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("UPDATE events SET contest_definition_version=1 WHERE id=$1")
        .bind(event)
        .execute(store.pool())
        .await
        .unwrap();
}

fn assert_policy_error(result: Result<qsonaut_protocol::QsoLog, sqlx::Error>) {
    let error = result.unwrap_err();
    assert!(
        matches!(error, sqlx::Error::Database(ref db) if db.code().as_deref() == Some("P1001")),
        "{error}"
    );
}

async fn assert_special_callsign_audit(store: &Store, event: Uuid, user: Uuid) {
    let special = store
        .create_special_callsign(
            &ManagedCallsignInput {
                event_id: event,
                callsign: format!("K{}", &Uuid::new_v4().simple().to_string()[..6]),
                authority: "contract-test".into(),
                authority_reference: "TEST-001".into(),
            },
            true,
            user,
        )
        .await
        .unwrap();
    assert_eq!(special.identity_type, "special");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM managed_callsign_audit WHERE callsign_id=$1 AND actor_user_id=$2 AND action='registered'",
        )
        .bind(special.id)
        .bind(user)
        .fetch_one(store.pool())
        .await
        .unwrap(),
        1
    );
}
