use chrono::{Duration, Utc};
use qsonaut_protocol::{
    EventInput, EventParticipantInput, EventStatus, ManagedCallsignInput, QsoLogInput,
};
use qsonaut_store::Store;
use uuid::Uuid;

#[tokio::test]
// Keep each database contract in one transactionally ordered scenario so a
// later assertion exercises the state transitions established above it.
#[allow(clippy::too_many_lines)]
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
    let operator_callsign = format!("T{}", user.simple()).to_uppercase();
    sqlx::query("INSERT INTO users(id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Contest test','unused','member')")
        .bind(user).bind(&operator_callsign).execute(store.pool()).await.unwrap();
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
    let participant_id = Uuid::new_v4();
    sqlx::query("INSERT INTO event_participants(id,event_id,user_id,club_id,callsign_id,operator_callsign,operating_callsign,role,status,starts_at,ends_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'operator','active',$8,$9)")
        .bind(participant_id).bind(event).bind(user).bind(club).bind(callsign_id)
        .bind(&operator_callsign).bind(&operating_callsign)
        .bind(now - Duration::hours(1)).bind(now + Duration::hours(1)).execute(store.pool()).await.unwrap();
    input.operating_callsign = Some(operating_callsign.clone());
    input.callsign_id = Some(callsign_id);
    input.occurred_at = now + Duration::hours(1);
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.occurred_at = now - Duration::hours(2);
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.occurred_at = now;
    let accepted_key = input.idempotency_key;
    let accepted = store.create_qso_log(user, &input).await.unwrap();
    assert_eq!(accepted.operator_callsign, operator_callsign);
    input.operating_callsign = Some("K0WRONG".into());
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.operating_callsign = Some(operating_callsign.clone());
    input.idempotency_key = accepted_key;
    sqlx::query("UPDATE users SET callsign=$2 WHERE id=$1")
        .bind(user)
        .bind(format!("X{}", user.simple()).to_uppercase())
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        store
            .create_qso_log(user, &input)
            .await
            .unwrap()
            .operator_callsign,
        operator_callsign
    );
    sqlx::query(
        "UPDATE club_members SET membership_status='lapsed' WHERE club_id=$1 AND user_id=$2",
    )
    .bind(club)
    .bind(user)
    .execute(store.pool())
    .await
    .unwrap();
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query(
        "UPDATE club_members SET membership_status='active' WHERE club_id=$1 AND user_id=$2",
    )
    .bind(club)
    .bind(user)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query("UPDATE club_members SET role='observer' WHERE club_id=$1 AND user_id=$2")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE event_participants SET role='observer' WHERE id=$1")
        .bind(participant_id)
        .execute(store.pool())
        .await
        .unwrap();
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("UPDATE club_members SET role='operator' WHERE club_id=$1 AND user_id=$2")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE event_participants SET role='operator',band='20' WHERE id=$1")
        .bind(participant_id)
        .execute(store.pool())
        .await
        .unwrap();
    input.band = "40m".into();
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    input.band = "20m".into();
    sqlx::query("UPDATE managed_callsigns SET expires_at=$2 WHERE id=$1")
        .bind(callsign_id)
        .bind(now - Duration::minutes(1))
        .execute(store.pool())
        .await
        .unwrap();
    input.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &input).await);
    sqlx::query("UPDATE managed_callsigns SET expires_at=NULL WHERE id=$1")
        .bind(callsign_id)
        .execute(store.pool())
        .await
        .unwrap();
    assert_database_policy_error(
        sqlx::query("UPDATE qso_logs SET band='40m' WHERE id=$1")
            .bind(accepted.id)
            .execute(store.pool())
            .await,
    );
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(event)
        .execute(store.pool())
        .await
        .unwrap();
    input.idempotency_key = accepted_key;
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
#[allow(clippy::too_many_lines)]
async fn new_user_and_club_callsigns_stay_in_the_managed_registry() {
    let Ok(url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
        eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping callsign registry contract");
        return;
    };
    let store = Store::connect(&url).await.unwrap();
    let user = Uuid::new_v4();
    let club = Uuid::new_v4();
    let user_call = format!("U{}", &user.simple().to_string()[..7]).to_uppercase();
    let club_call = format!("C{}", &club.simple().to_string()[..7]).to_uppercase();
    sqlx::query("INSERT INTO users(id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Registry test','unused','member')")
        .bind(user).bind(&user_call).execute(store.pool()).await.unwrap();
    sqlx::query("INSERT INTO clubs(id,name,callsign) VALUES ($1,$2,$3)")
        .bind(club)
        .bind(format!("Registry {club}"))
        .bind(&club_call)
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM managed_callsigns WHERE owner_user_id=$1 AND callsign=$2 AND identity_type='personal'")
            .bind(user).bind(&user_call).fetch_one(store.pool()).await.unwrap(),
        1
    );
    let personal_identity = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM managed_callsigns WHERE owner_user_id=$1 AND identity_type='personal'",
    )
    .bind(user)
    .fetch_one(store.pool())
    .await
    .unwrap();
    let club_identity = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,callsign FROM managed_callsigns WHERE club_id=$1 AND identity_type='club'",
    )
    .bind(club)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(club_identity.1, club_call);
    let renamed_call = format!("R{}", &club.simple().to_string()[..7]).to_uppercase();
    sqlx::query("UPDATE clubs SET callsign=$2 WHERE id=$1")
        .bind(club)
        .bind(&renamed_call)
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT callsign FROM managed_callsigns WHERE id=$1")
            .bind(club_identity.0)
            .fetch_one(store.pool())
            .await
            .unwrap(),
        renamed_call
    );

    let now = Utc::now();
    let mut personal_log = QsoLogInput {
        event_id: None,
        operating_callsign: None,
        callsign_id: None,
        visibility: "private".into(),
        visibility_club_id: None,
        idempotency_key: Uuid::new_v4(),
        callsign: "W1AW".into(),
        band: "20m".into(),
        mode: "SSB".into(),
        frequency_hz: Some(14_250_000),
        occurred_at: now,
        rst_sent: Some("59".into()),
        rst_received: Some("59".into()),
        exchange: serde_json::json!({}),
        points: 0,
        source: "registry-contract".into(),
    };
    let inferred = store.create_qso_log(user, &personal_log).await.unwrap();
    assert_eq!(inferred.callsign_id, Some(personal_identity));
    assert_eq!(
        inferred.operating_callsign.as_deref(),
        Some(user_call.as_str())
    );

    sqlx::query("INSERT INTO club_members(club_id,user_id,role) VALUES ($1,$2,'observer')")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    personal_log.callsign_id = Some(club_identity.0);
    personal_log.operating_callsign = Some(renamed_call.clone());
    personal_log.idempotency_key = Uuid::new_v4();
    assert_policy_error(store.create_qso_log(user, &personal_log).await);

    sqlx::query("UPDATE club_members SET role='operator' WHERE club_id=$1 AND user_id=$2")
        .bind(club)
        .bind(user)
        .execute(store.pool())
        .await
        .unwrap();
    personal_log.idempotency_key = Uuid::new_v4();
    assert!(store.create_qso_log(user, &personal_log).await.is_ok());

    let managed_event = store
        .create_managed_event(
            &EventInput {
                club_id: club,
                name: "Managed special call".into(),
                contest_name: String::new(),
                special_callsign: Some(format!("S{}", &club.simple().to_string()[..7])),
                starts_at: now - Duration::minutes(5),
                ends_at: now + Duration::hours(1),
                status: EventStatus::Draft,
                contest_template_id: None,
                contest_config: serde_json::json!({}),
            },
            user,
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM managed_callsigns c JOIN managed_callsign_audit a ON a.callsign_id=c.id WHERE c.event_id=$1 AND c.identity_type='special' AND c.status='pending' AND c.verification_status='pending' AND a.actor_user_id=$2 AND a.action='registered'")
            .bind(managed_event.id).bind(user).fetch_one(store.pool()).await.unwrap(),
        1
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
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
    let participant = store
        .create_event_participant(
            event,
            &EventParticipantInput {
                user_id: user,
                callsign_id,
                operator_callsign: format!("U{}", user.simple()).to_uppercase(),
                role: "operator".into(),
                status: "active".into(),
                starts_at: None,
                ends_at: None,
                station_label: "Radio 1".into(),
                band: None,
                mode: None,
            },
            Some(user),
        )
        .await
        .unwrap();
    assert!(participant.starts_at.is_some());
    assert!(participant.ends_at.is_some());
    assert_eq!(
        store
            .events()
            .await
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.id == event)
            .unwrap()
            .participant_count,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_participant_audit WHERE participant_id=$1 AND actor_user_id=$2 AND action='created'")
            .bind(participant.id).bind(user).fetch_one(store.pool()).await.unwrap(),
        1
    );
    let updated_participant = store
        .update_event_participant(
            event,
            participant.id,
            &EventParticipantInput {
                user_id: user,
                callsign_id,
                operator_callsign: format!("U{}", user.simple()).to_uppercase(),
                role: "operator".into(),
                status: "active".into(),
                starts_at: None,
                ends_at: None,
                station_label: "Radio 1".into(),
                band: None,
                mode: None,
            },
            user,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_participant.station_label, "Radio 1");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_participant_audit WHERE participant_id=$1 AND actor_user_id=$2 AND action='changed'")
            .bind(participant.id).bind(user).fetch_one(store.pool()).await.unwrap(),
        1
    );
    assert_database_policy_error(
        sqlx::query("UPDATE events SET ends_at=ends_at + interval '1 hour' WHERE id=$1")
            .bind(event)
            .execute(store.pool())
            .await,
    );
    assert_database_policy_error(
        sqlx::query("UPDATE event_participants SET band='30m' WHERE id=$1")
            .bind(participant.id)
            .execute(store.pool())
            .await,
    );
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
    input.callsign = "W2AW".into();
    input.band = "70cm".into();
    input.idempotency_key = Uuid::new_v4();
    let centimeter_band = store.create_qso_log(user, &input).await.unwrap();
    assert_eq!(centimeter_band.band, "70CM");
    assert_eq!(centimeter_band.points, 1);
    input.callsign = "W3AW".into();
    input.band = "40m".into();
    input.mode = "USB".into();
    input.idempotency_key = Uuid::new_v4();
    assert_eq!(store.create_qso_log(user, &input).await.unwrap().points, 1);
    input.callsign = "W1AW".into();
    input.band = "20m".into();
    input.mode = "FT8".into();
    input.idempotency_key = Uuid::new_v4();
    let duplicate = store.create_qso_log(user, &input).await.unwrap();
    assert!(duplicate.is_duplicate);
    assert_eq!(duplicate.points, 0);
    let event_score = store.event_score(event).await.unwrap();
    assert_eq!(event_score.total_points, 3);
    assert_eq!(event_score.qso_count, 4);
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
    assert_database_policy_error(result);
}

fn assert_database_policy_error<T>(result: Result<T, sqlx::Error>) {
    let error = result.err().expect("operation should be rejected");
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
            false,
            user,
        )
        .await
        .unwrap();
    assert_eq!(special.identity_type, "special");
    assert_eq!(special.status, "pending");
    let approved = store
        .update_special_callsign_status(special.id, "active", "verified", "approved", user)
        .await
        .unwrap();
    assert_eq!(approved.status, "active");
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
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM managed_callsign_audit WHERE callsign_id=$1 AND actor_user_id=$2 AND action='approved'",
        )
        .bind(special.id)
        .bind(user)
        .fetch_one(store.pool())
        .await
        .unwrap(),
        1
    );
    assert_database_policy_error(
        sqlx::query("UPDATE managed_callsign_audit SET details='{}' WHERE callsign_id=$1")
            .bind(special.id)
            .execute(store.pool())
            .await,
    );
}
