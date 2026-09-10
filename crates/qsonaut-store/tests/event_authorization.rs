use chrono::{Duration, Utc};
use qsonaut_protocol::QsoLogInput;
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

fn assert_policy_error(result: Result<qsonaut_protocol::QsoLog, sqlx::Error>) {
    let error = result.unwrap_err();
    assert!(
        matches!(error, sqlx::Error::Database(ref db) if db.code().as_deref() == Some("P1001")),
        "{error}"
    );
}
