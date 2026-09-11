//! Versioned HTTP API assembly for `QSONaut` Server.

mod access;
mod auth;
mod error;
mod management;
mod realtime;

use axum::{
    Json, Router,
    routing::{delete, get, patch, post},
};
use qsonaut_protocol::{
    API_VERSION, AccessCallsignLookup, AccessChallenge, AccessDecisionResult, AccessRequest,
    AccessRequestDecisionInput, AccessRequestInput, ActivitySummary, ActivityVisibility,
    ActivityVisibilityInput, ApiError, BootstrapRequest, ChannelMessage, Club, ClubInput,
    ClubJoinDecisionInput, ClubJoinRequest, ClubMembership, ClubMembershipInput, ContestTemplate,
    Credentials, CurrentUser, DeviceCredentials, DeviceRegistration, DeviceToken,
    DeviceTokenRecord, DiagnosticReport, DiagnosticReportInput, Event, EventInput,
    EventParticipant, EventParticipantInput, EventScore, EventStatus, EventStatusInput,
    EventUpdateInput, HealthResponse, ManagedCallsign, ManagedCallsignInput,
    ManagedCallsignStatusInput, MemberClubRole, MemberDetail, MemberInput, MemberUpdateInput,
    PasswordResetInput, ProfileUpdateInput, QsoLog, QsoLogInput, ServerCapabilities, ServiceInfo,
    ServiceStatus, SetupStatus, ShareLink, ShareLinkInput, ShareLinkRecord, SharedQsoDetail,
    StationPresence, StationPresenceInput, UserProfile,
};
use qsonaut_store::Store;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health, service_info, capabilities, access::challenge, access::lookup_callsign, access::submit, access::list, access::decide,
        auth::setup_status, auth::bootstrap, auth::login, auth::me, auth::update_me, auth::profile, auth::update_profile, auth::refresh_hamdb_profile, auth::update_my_password, auth::logout,
        auth::device_login, auth::register_session_device, auth::session_devices,
        auth::reissue_session_device, auth::revoke_session_device, auth::revoke_device,
        management::members, management::create_member, management::member_detail, management::update_member, management::reset_member_password,
        management::club_members, management::set_club_member, management::remove_club_member,
        management::clubs, management::create_club, management::update_club,
        management::request_club_join, management::club_join_requests, management::review_club_join_request,
        management::contest_templates, management::identities, management::register_special_callsign, management::update_special_callsign_status,
        management::events, management::create_event, management::update_event, management::set_event_status,
        management::event_participants, management::create_event_participant, management::update_event_participant, management::event_score,
        management::stations, management::publish_station_presence, management::channel_messages,
        management::logs, management::collect_log, management::diagnostics, management::export_diagnostics, management::purge_retained_artifacts, management::activity_summary, management::activity_visibility, management::set_activity_visibility,
        management::create_log_share, management::shared_log, management::revoke_log_share, management::log_shares
    ),
    components(schemas(
        HealthResponse, ServiceInfo, ServerCapabilities, ServiceStatus, ApiError, SetupStatus, Credentials,
        DeviceCredentials, DeviceRegistration, DeviceToken, DeviceTokenRecord,
        BootstrapRequest, CurrentUser, AccessChallenge, AccessCallsignLookup, AccessRequest,
        AccessDecisionResult, AccessRequestInput, AccessRequestDecisionInput, MemberInput,
        ClubMembership, ClubMembershipInput, ClubJoinRequest, ClubJoinDecisionInput,
        MemberUpdateInput, PasswordResetInput, ProfileUpdateInput, UserProfile, ActivitySummary, ActivityVisibility, ActivityVisibilityInput, MemberClubRole, MemberDetail, ShareLinkInput, ShareLinkRecord,
        ShareLink, SharedQsoDetail,
        Club, ClubInput, ContestTemplate, EventStatus, Event, EventInput, EventUpdateInput, EventStatusInput, ChannelMessage,
        ManagedCallsign, ManagedCallsignInput, ManagedCallsignStatusInput, EventParticipant, EventParticipantInput, EventScore,
        StationPresence, StationPresenceInput, QsoLog, QsoLogInput, DiagnosticReport, DiagnosticReportInput
    )),
    tags(
        (name = "service", description = "Service discovery and readiness"),
        (name = "access", description = "Public access requests and administrator review"),
        (name = "authentication", description = "Bootstrap and browser sessions"),
        (name = "management", description = "Club, member, contest, and event management"),
        (name = "activity", description = "QSONaut station presence and collected logs")
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub(crate) store: Store,
    pub(crate) secure_cookies: bool,
    pub(crate) channel_messages: tokio::sync::broadcast::Sender<ChannelMessage>,
    pub(crate) policy: ServerPolicy,
}

/// Public/hosted deployment policy. The public constructor is the safe default;
/// proprietary deployments can supply a policy without forking the API.
#[derive(Clone, Debug)]
pub struct ServerPolicy {
    pub edition: String,
    pub max_clubs: Option<i64>,
    pub features: Vec<String>,
}

impl ServerPolicy {
    #[must_use]
    pub fn public() -> Self {
        Self::custom(
            "community",
            Some(5),
            vec![
                "personal activity".to_owned(),
                "shared clubs and activities".to_owned(),
                "contest and event setup".to_owned(),
            ],
        )
    }

    /// Create a deployment policy with an optional positive club limit.
    ///
    /// # Panics
    ///
    /// Panics if `max_clubs` is present and is not positive.
    pub fn custom(
        edition: impl Into<String>,
        max_clubs: Option<i64>,
        features: Vec<String>,
    ) -> Self {
        assert!(max_clubs.is_none_or(|limit| limit > 0));
        Self {
            edition: edition.into(),
            max_clubs,
            features,
        }
    }

    #[must_use]
    pub fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            edition: self.edition.clone(),
            max_clubs: self.max_clubs,
            features: self.features.clone(),
        }
    }

    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|item| item == feature)
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/info", get(service_info))
        .route("/api/v1/openapi.json", get(openapi))
}

#[allow(clippy::too_many_lines)]
pub fn router_with_store(store: Store, secure_cookies: bool) -> Router {
    router_with_store_and_policy(store, secure_cookies, ServerPolicy::public())
}

/// Assemble the public API with an extension-provided deployment policy.
#[allow(clippy::too_many_lines)]
pub fn router_with_store_and_policy(
    store: Store,
    secure_cookies: bool,
    policy: ServerPolicy,
) -> Router {
    let (channel_messages, _) = tokio::sync::broadcast::channel(256);
    let state = AppState {
        store,
        secure_cookies,
        channel_messages,
        policy,
    };
    let management = Router::<AppState>::new()
        .route("/api/v1/capabilities", get(capabilities))
        .route("/api/v1/access/challenge", get(access::challenge))
        .route(
            "/api/v1/access/lookup/{callsign}",
            get(access::lookup_callsign),
        )
        .route("/api/v1/access/request", post(access::submit))
        .route("/api/v1/access/requests", get(access::list))
        .route(
            "/api/v1/access/requests/{request_id}",
            patch(access::decide),
        )
        .route(
            "/api/v1/auth/setup",
            get(auth::setup_status).post(auth::bootstrap),
        )
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/me", patch(auth::update_me))
        .route(
            "/api/v1/auth/profile",
            get(auth::profile).patch(auth::update_profile),
        )
        .route(
            "/api/v1/auth/profile/hamdb",
            post(auth::refresh_hamdb_profile),
        )
        .route(
            "/api/v1/activity/visibility",
            get(management::activity_visibility).put(management::set_activity_visibility),
        )
        .route("/api/v1/shares", get(management::log_shares))
        .route(
            "/api/v1/diagnostics/export",
            get(management::export_diagnostics),
        )
        .route(
            "/api/v1/diagnostics/retention/purge",
            post(management::purge_retained_artifacts),
        )
        .route("/api/v1/auth/me/password", post(auth::update_my_password))
        .route(
            "/api/v1/auth/device",
            post(auth::device_login).delete(auth::revoke_device),
        )
        .route(
            "/api/v1/auth/device/session",
            post(auth::register_session_device),
        )
        .route("/api/v1/auth/devices", get(auth::session_devices))
        .route(
            "/api/v1/auth/devices/{token_id}",
            axum::routing::delete(auth::revoke_session_device),
        )
        .route(
            "/api/v1/auth/devices/{token_id}/reissue",
            post(auth::reissue_session_device),
        )
        .route("/api/v1/ws", get(realtime::connect))
        .route(
            "/api/v1/clubs",
            get(management::clubs).post(management::create_club),
        )
        .route("/api/v1/clubs/{club_id}", patch(management::update_club))
        .route(
            "/api/v1/members",
            get(management::members).post(management::create_member),
        )
        .route(
            "/api/v1/members/{member_id}",
            get(management::member_detail).patch(management::update_member),
        )
        .route(
            "/api/v1/members/{member_id}/password",
            post(management::reset_member_password),
        )
        .route(
            "/api/v1/clubs/{club_id}/members",
            get(management::club_members).put(management::set_club_member),
        )
        .route(
            "/api/v1/clubs/{club_id}/members/{member_id}",
            axum::routing::delete(management::remove_club_member),
        )
        .route(
            "/api/v1/events",
            get(management::events).post(management::create_event),
        )
        .route(
            "/api/v1/events/{event_id}/status",
            patch(management::set_event_status),
        )
        .route("/api/v1/events/{event_id}", patch(management::update_event))
        .route(
            "/api/v1/events/{event_id}/participants",
            get(management::event_participants).post(management::create_event_participant),
        )
        .route(
            "/api/v1/events/{event_id}/participants/{participant_id}",
            patch(management::update_event_participant),
        )
        .route(
            "/api/v1/events/{event_id}/score",
            get(management::event_score),
        )
        .route(
            "/api/v1/clubs/{club_id}/join-requests",
            get(management::club_join_requests).post(management::request_club_join),
        )
        .route(
            "/api/v1/clubs/{club_id}/join-requests/{request_id}",
            patch(management::review_club_join_request),
        )
        .route(
            "/api/v1/contest-templates",
            get(management::contest_templates),
        )
        .route("/api/v1/identities", get(management::identities))
        .route(
            "/api/v1/identities/special",
            post(management::register_special_callsign),
        )
        .route(
            "/api/v1/identities/{identity_id}/status",
            patch(management::update_special_callsign_status),
        )
        .route("/api/v1/stations", get(management::stations))
        .route(
            "/api/v1/channel-messages",
            get(management::channel_messages),
        )
        .route(
            "/api/v1/stations/presence",
            axum::routing::put(management::publish_station_presence),
        )
        .route(
            "/api/v1/logs",
            get(management::logs).post(management::collect_log),
        )
        .route(
            "/api/v1/activity/summary",
            get(management::activity_summary),
        )
        .route(
            "/api/v1/logs/{log_id}/share",
            post(management::create_log_share),
        )
        .route(
            "/api/v1/shares/{share_id}",
            delete(management::revoke_log_share),
        )
        .route("/api/v1/share/{token}", get(management::shared_log))
        .route("/api/v1/diagnostics", get(management::diagnostics))
        .with_state(state);
    router().merge(management)
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "service",
    responses((status = 200, description = "Service is ready", body = HealthResponse))
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: ServiceStatus::Ready,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/info",
    tag = "service",
    responses((status = 200, description = "Service identity and scope", body = ServiceInfo))
)]
async fn service_info() -> Json<ServiceInfo> {
    Json(ServiceInfo {
        name: "QSONaut Server".to_owned(),
        api_version: API_VERSION.to_owned(),
        purpose: "Group-event management, coordination, and synchronization".to_owned(),
        manages: vec![
            "clubs and shared activity spaces".to_owned(),
            "events and contest setup".to_owned(),
            "shared logs and reports".to_owned(),
            "chat and presence".to_owned(),
            "N3FJP integration".to_owned(),
        ],
        excludes: vec![
            "radio control".to_owned(),
            "audio".to_owned(),
            "DSP and decoding".to_owned(),
            "PTT and transmit activity".to_owned(),
        ],
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/capabilities",
    tag = "service",
    responses((status = 200, description = "Deployment edition and feature policy", body = ServerCapabilities))
)]
async fn capabilities(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ServerCapabilities> {
    Json(state.policy.capabilities())
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use chrono::{Duration, Utc};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn health_contract_is_versioned_and_ready() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), 200);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).expect("valid JSON");
        assert_eq!(value["status"], "ready");
    }

    #[tokio::test]
    async fn info_contract_preserves_the_activity_boundary() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/info")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).expect("valid JSON");
        let excluded = value["excludes"].as_array().expect("excludes array");
        assert!(
            excluded
                .iter()
                .any(|item| item == "PTT and transmit activity")
        );
    }

    #[tokio::test]
    async fn member_can_read_only_their_own_station_presence() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping station API contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let user_id = Uuid::new_v4();
        let callsign = format!("T{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "station-api-contract-password";
        let hash = auth::hash_password(password.to_owned()).await.expect("hash password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Station API contract',$3,'member')")
            .bind(user_id).bind(&callsign).bind(hash).execute(store.pool()).await.expect("insert API test user");
        let instance_id = Uuid::new_v4();
        sqlx::query("INSERT INTO station_presence (id,user_id,instance_id,station_label,qsonaut_version,platform,status,metadata) VALUES ($1,$2,$3,'owned station','test','linux','online','{}')")
            .bind(Uuid::new_v4()).bind(user_id).bind(instance_id).execute(store.pool()).await.expect("insert owned station");
        let login = router_with_store(store.clone(), false).oneshot(Request::builder().method("POST").uri("/api/v1/auth/login").header("content-type", "application/json").body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#))).expect("login request")).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie").to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();
        let response = router_with_store(store, false).oneshot(Request::builder().uri("/api/v1/stations").header("cookie", cookie).body(Body::empty()).expect("station request")).await.expect("station response");
        assert_eq!(response.status(), 200);
        let body = response.into_body().collect().await.expect("station body").to_bytes();
        let stations: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("station JSON");
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0]["instance_id"], instance_id.to_string());
    }

    #[tokio::test]
    async fn administrator_can_review_global_station_and_diagnostic_data() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping administrator API contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let member_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let member_callsign = format!("M{}", &member_id.simple().to_string()[..8]).to_ascii_uppercase();
        let admin_callsign = format!("A{}", &admin_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "global-review-contract-password";
        let member_hash = auth::hash_password(password.to_owned()).await.expect("hash member password");
        let admin_hash = auth::hash_password(password.to_owned()).await.expect("hash admin password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Review member',$3,'member'),($4,$5,'Review administrator',$6,'administrator')")
            .bind(member_id).bind(&member_callsign).bind(member_hash)
            .bind(admin_id).bind(&admin_callsign).bind(admin_hash)
            .execute(store.pool()).await.expect("insert API test users");

        let member_instance = Uuid::new_v4();
        let admin_instance = Uuid::new_v4();
        for (user_id, instance_id, label) in [
            (member_id, member_instance, "member station"),
            (admin_id, admin_instance, "administrator station"),
        ] {
            sqlx::query("INSERT INTO station_presence (id,user_id,instance_id,station_label,qsonaut_version,platform,status,metadata) VALUES ($1,$2,$3,$4,'test','linux','online','{}')")
                .bind(Uuid::new_v4()).bind(user_id).bind(instance_id).bind(label)
                .execute(store.pool()).await.expect("insert station presence");
        }
        let member_report = store.create_diagnostic_report(member_id, &DiagnosticReportInput {
            instance_id: member_instance,
            category: "audio".to_owned(),
            summary: "member report".to_owned(),
            payload: serde_json::json!({"source": "api-contract"}),
        }).await.expect("create member diagnostic");
        let admin_report = store.create_diagnostic_report(admin_id, &DiagnosticReportInput {
            instance_id: admin_instance,
            category: "radio".to_owned(),
            summary: "administrator report".to_owned(),
            payload: serde_json::json!({"source": "api-contract"}),
        }).await.expect("create administrator diagnostic");

        async fn login_cookie(store: Store, callsign: &str, password: &str) -> String {
            let response = router_with_store(store, false).oneshot(
                Request::builder().method("POST").uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                    .expect("login request")
            ).await.expect("login response");
            assert_eq!(response.status(), 200);
            response.headers().get("set-cookie").expect("session cookie")
                .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned()
        }

        let member_cookie = login_cookie(store.clone(), &member_callsign, password).await;
        let member_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/diagnostics").header("cookie", member_cookie)
                .body(Body::empty()).expect("member diagnostics request")
        ).await.expect("member diagnostics response");
        assert_eq!(member_response.status(), 200);
        let member_body = member_response.into_body().collect().await.expect("member diagnostics body").to_bytes();
        let member_reports: Vec<DiagnosticReport> = serde_json::from_slice(&member_body).expect("member diagnostics JSON");
        assert!(member_reports.iter().all(|report| report.id == member_report.id));

        let admin_cookie = login_cookie(store.clone(), &admin_callsign, password).await;
        let admin_station_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/stations").header("cookie", &admin_cookie)
                .body(Body::empty()).expect("administrator station request")
        ).await.expect("administrator station response");
        assert_eq!(admin_station_response.status(), 200);
        let station_body = admin_station_response.into_body().collect().await.expect("station body").to_bytes();
        let stations: Vec<StationPresence> = serde_json::from_slice(&station_body).expect("station JSON");
        assert!(stations.iter().any(|station| station.instance_id == member_instance));
        assert!(stations.iter().any(|station| station.instance_id == admin_instance));

        let admin_diagnostic_response = router_with_store(store, false).oneshot(
            Request::builder().uri("/api/v1/diagnostics").header("cookie", admin_cookie)
                .body(Body::empty()).expect("administrator diagnostics request")
        ).await.expect("administrator diagnostics response");
        assert_eq!(admin_diagnostic_response.status(), 200);
        let diagnostic_body = admin_diagnostic_response.into_body().collect().await.expect("diagnostics body").to_bytes();
        let reports: Vec<DiagnosticReport> = serde_json::from_slice(&diagnostic_body).expect("diagnostics JSON");
        assert!(reports.iter().any(|report| report.id == member_report.id));
        assert!(reports.iter().any(|report| report.id == admin_report.id));
    }

    #[tokio::test]
    async fn member_sees_events_only_for_active_clubs_they_belong_to() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping event visibility contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let member_id = Uuid::new_v4();
        let other_owner_id = Uuid::new_v4();
        let member_callsign = format!("E{}", &member_id.simple().to_string()[..8]).to_ascii_uppercase();
        let other_callsign = format!("O{}", &other_owner_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "event-visibility-contract-password";
        let member_hash = auth::hash_password(password.to_owned()).await.expect("hash member password");
        let other_hash = auth::hash_password(password.to_owned()).await.expect("hash other password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Event member',$3,'member'),($4,$5,'Other club owner',$6,'member')")
            .bind(member_id).bind(&member_callsign).bind(member_hash)
            .bind(other_owner_id).bind(&other_callsign).bind(other_hash)
            .execute(store.pool()).await.expect("insert event test users");

        let member_club = store.create_club(&ClubInput {
            name: format!("Visible Club {}", &member_id.simple().to_string()[..6]),
            callsign: None,
            description: "club visible to the member".to_owned(),
        }, member_id).await.expect("create member club");
        let other_club = store.create_club(&ClubInput {
            name: format!("Hidden Club {}", &other_owner_id.simple().to_string()[..6]),
            callsign: None,
            description: "club outside the member scope".to_owned(),
        }, other_owner_id).await.expect("create unrelated club");
        let now = Utc::now();
        let visible_event = store.create_event(&EventInput {
            club_id: member_club.id,
            name: "Member-visible contest".to_owned(),
            contest_name: "Test contest".to_owned(),
            special_callsign: None,
            starts_at: now,
            ends_at: now + Duration::hours(2),
            status: EventStatus::Scheduled,
            contest_template_id: None,
            contest_config: serde_json::json!({}),
        }).await.expect("create visible event");
        let hidden_event = store.create_event(&EventInput {
            club_id: other_club.id,
            name: "Unrelated contest".to_owned(),
            contest_name: "Test contest".to_owned(),
            special_callsign: None,
            starts_at: now,
            ends_at: now + Duration::hours(2),
            status: EventStatus::Scheduled,
            contest_template_id: None,
            contest_config: serde_json::json!({}),
        }).await.expect("create hidden event");

        let login = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"callsign":"{member_callsign}","password":"{password}"}}"#)))
                .expect("login request")
        ).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie")
            .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();
        let response = router_with_store(store, false).oneshot(
            Request::builder().uri("/api/v1/events").header("cookie", cookie)
                .body(Body::empty()).expect("events request")
        ).await.expect("events response");
        assert_eq!(response.status(), 200);
        let body = response.into_body().collect().await.expect("events body").to_bytes();
        let events: Vec<Event> = serde_json::from_slice(&body).expect("events JSON");
        assert!(events.iter().any(|event| event.id == visible_event.id));
        assert!(!events.iter().any(|event| event.id == hidden_event.id));
        assert!(events.iter().all(|event| event.club_id == member_club.id));
    }

    #[tokio::test]
    async fn member_cannot_access_global_administrator_routes() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping administrator boundary contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let user_id = Uuid::new_v4();
        let callsign = format!("G{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "administrator-boundary-contract-password";
        let hash = auth::hash_password(password.to_owned()).await.expect("hash password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Boundary member',$3,'member')")
            .bind(user_id).bind(&callsign).bind(hash).execute(store.pool()).await
            .expect("insert boundary test user");
        let login = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                .expect("login request")
        ).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie")
            .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();

        for (method, path) in [
            ("GET", "/api/v1/members"),
            ("GET", "/api/v1/diagnostics/export"),
            ("POST", "/api/v1/diagnostics/retention/purge"),
        ] {
            let response = router_with_store(store.clone(), false).oneshot(
                Request::builder().method(method).uri(path).header("cookie", &cookie)
                    .body(Body::empty()).expect("administrator route request")
            ).await.expect("administrator route response");
            assert_eq!(response.status(), 403, "member could access {path}");
        }
        let unauthenticated = router_with_store(store, false).oneshot(
            Request::builder().uri("/api/v1/stations").body(Body::empty()).expect("unauthenticated request")
        ).await.expect("unauthenticated response");
        assert_eq!(unauthenticated.status(), 401);
    }

    #[tokio::test]
    async fn member_can_update_profile_and_create_a_club_visible_in_workspace() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping profile workspace contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let policy = ServerPolicy::custom("test", None, Vec::new());
        let user_id = Uuid::new_v4();
        let callsign = format!("P{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
        let club_callsign = format!("w{}", &user_id.simple().to_string()[..6]);
        let password = "profile-workspace-contract-password";
        let hash = auth::hash_password(password.to_owned()).await.expect("hash password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Original profile',$3,'member')")
            .bind(user_id).bind(&callsign).bind(hash).execute(store.pool()).await
            .expect("insert profile test user");
        let login = router_with_store_and_policy(store.clone(), false, policy.clone()).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                .expect("login request")
        ).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie")
            .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();

        let profile_update = router_with_store_and_policy(store.clone(), false, policy.clone()).oneshot(
            Request::builder().method("PATCH").uri("/api/v1/auth/me")
                .header("content-type", "application/json").header("cookie", &cookie)
                .body(Body::from(r#"{"display_name":"Updated profile"}"#))
                .expect("profile update request")
        ).await.expect("profile update response");
        assert_eq!(profile_update.status(), 200);
        let updated_user: CurrentUser = serde_json::from_slice(
            &profile_update.into_body().collect().await.expect("profile update body").to_bytes()
        ).expect("updated user JSON");
        assert_eq!(updated_user.display_name, "Updated profile");

        let club_response = router_with_store_and_policy(store.clone(), false, policy.clone()).oneshot(
            Request::builder().method("POST").uri("/api/v1/clubs")
                .header("content-type", "application/json").header("cookie", &cookie)
                .body(Body::from(format!(r#"{{"name":"Workspace Club {user_id}","callsign":"{club_callsign}","description":"A club for workspace testing"}}"#)))
                .expect("club creation request")
        ).await.expect("club creation response");
        assert_eq!(club_response.status(), 200);
        let created_club: Club = serde_json::from_slice(
            &club_response.into_body().collect().await.expect("club body").to_bytes()
        ).expect("created club JSON");
        assert_eq!(created_club.callsign.as_deref(), Some(club_callsign.to_ascii_uppercase().as_str()));
        assert_eq!(created_club.my_role.as_deref(), Some("owner"));

        let clubs_response = router_with_store_and_policy(store.clone(), false, policy.clone()).oneshot(
            Request::builder().uri("/api/v1/clubs").header("cookie", &cookie)
                .body(Body::empty()).expect("clubs request")
        ).await.expect("clubs response");
        assert_eq!(clubs_response.status(), 200);
        let clubs: Vec<Club> = serde_json::from_slice(
            &clubs_response.into_body().collect().await.expect("clubs body").to_bytes()
        ).expect("clubs JSON");
        let workspace_club = clubs.iter().find(|club| club.id == created_club.id).expect("created club in workspace");
        assert_eq!(workspace_club.my_membership_status.as_deref(), Some("active"));
        assert!(workspace_club.can_manage);

        let me_response = router_with_store_and_policy(store.clone(), false, policy).oneshot(
            Request::builder().uri("/api/v1/auth/me").header("cookie", &cookie)
                .body(Body::empty()).expect("me request")
        ).await.expect("me response");
        assert_eq!(me_response.status(), 200);
        let me: CurrentUser = serde_json::from_slice(
            &me_response.into_body().collect().await.expect("me body").to_bytes()
        ).expect("me JSON");
        assert_eq!(me.display_name, "Updated profile");

        let profile_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/auth/profile").header("cookie", &cookie)
                .body(Body::empty()).expect("profile request")
        ).await.expect("profile response");
        assert_eq!(profile_response.status(), 200);
        let profile: UserProfile = serde_json::from_slice(
            &profile_response.into_body().collect().await.expect("profile body").to_bytes()
        ).expect("profile JSON");
        assert_eq!(profile.user.display_name, "Updated profile");
    }

    #[tokio::test]
    async fn member_can_request_club_join_and_owner_can_approve_it() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping club join contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let owner_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let owner_callsign = format!("C{}", &owner_id.simple().to_string()[..8]).to_ascii_uppercase();
        let member_callsign = format!("J{}", &member_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "club-join-contract-password";
        let owner_hash = auth::hash_password(password.to_owned()).await.expect("hash owner password");
        let member_hash = auth::hash_password(password.to_owned()).await.expect("hash member password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Join owner',$3,'member'),($4,$5,'Join member',$6,'member')")
            .bind(owner_id).bind(&owner_callsign).bind(owner_hash)
            .bind(member_id).bind(&member_callsign).bind(member_hash)
            .execute(store.pool()).await.expect("insert join test users");
        let club = store.create_club(&ClubInput {
            name: format!("Join Club {}", &owner_id.simple().to_string()[..6]),
            callsign: None,
            description: "join workflow contract".to_owned(),
        }, owner_id).await.expect("create join club");

        async fn login(store: Store, callsign: &str, password: &str) -> String {
            let response = router_with_store(store, false).oneshot(
                Request::builder().method("POST").uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                    .expect("login request")
            ).await.expect("login response");
            assert_eq!(response.status(), 200);
            response.headers().get("set-cookie").expect("session cookie")
                .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned()
        }

        let member_cookie = login(store.clone(), &member_callsign, password).await;
        let request_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/v1/clubs/{}/join-requests", club.id))
                .header("cookie", &member_cookie).body(Body::empty()).expect("join request")
        ).await.expect("join response");
        assert_eq!(request_response.status(), 200);
        let join_request: ClubJoinRequest = serde_json::from_slice(
            &request_response.into_body().collect().await.expect("join body").to_bytes()
        ).expect("join request JSON");
        assert_eq!(join_request.status, "pending");

        let owner_cookie = login(store.clone(), &owner_callsign, password).await;
        let pending_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri(format!("/api/v1/clubs/{}/join-requests", club.id))
                .header("cookie", &owner_cookie).body(Body::empty()).expect("pending request")
        ).await.expect("pending response");
        assert_eq!(pending_response.status(), 200);
        let pending: Vec<ClubJoinRequest> = serde_json::from_slice(
            &pending_response.into_body().collect().await.expect("pending body").to_bytes()
        ).expect("pending JSON");
        assert!(pending.iter().any(|item| item.id == join_request.id));

        let decision_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("PATCH")
                .uri(format!("/api/v1/clubs/{}/join-requests/{}", club.id, join_request.id))
                .header("content-type", "application/json").header("cookie", &owner_cookie)
                .body(Body::from(r#"{"decision":"approve","role":"operator"}"#))
                .expect("decision request")
        ).await.expect("decision response");
        assert_eq!(decision_response.status(), 200);
        let approved: ClubJoinRequest = serde_json::from_slice(
            &decision_response.into_body().collect().await.expect("decision body").to_bytes()
        ).expect("decision JSON");
        assert_eq!(approved.status, "approved");
        assert_eq!(store.club_role(club.id, member_id).await.expect("membership lookup").as_deref(), Some("operator"));
    }

    #[tokio::test]
    async fn member_can_submit_log_and_change_activity_visibility() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping activity sharing contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let user_id = Uuid::new_v4();
        let callsign = format!("S{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "activity-sharing-contract-password";
        let hash = auth::hash_password(password.to_owned()).await.expect("hash password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Sharing member',$3,'member')")
            .bind(user_id).bind(&callsign).bind(hash).execute(store.pool()).await
            .expect("insert sharing test user");
        let login = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                .expect("login request")
        ).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie")
            .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();
        let idempotency_key = Uuid::new_v4();
        let occurred_at = Utc::now();
        let log_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/logs")
                .header("content-type", "application/json").header("cookie", &cookie)
                .body(Body::from(format!(r#"{{"idempotency_key":"{idempotency_key}","callsign":"k1abc","band":"20m","mode":"FT8","occurred_at":"{}","exchange":{{"section":"OR"}},"source":"api-contract"}}"#, occurred_at.to_rfc3339())))
                .expect("log request")
        ).await.expect("log response");
        assert_eq!(log_response.status(), 200);
        let log: QsoLog = serde_json::from_slice(
            &log_response.into_body().collect().await.expect("log body").to_bytes()
        ).expect("log JSON");
        assert_eq!(log.user_id, user_id);
        assert_eq!(log.mode, "FT8");

        let logs_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/logs").header("cookie", &cookie)
                .body(Body::empty()).expect("logs request")
        ).await.expect("logs response");
        assert_eq!(logs_response.status(), 200);
        let logs: Vec<QsoLog> = serde_json::from_slice(
            &logs_response.into_body().collect().await.expect("logs body").to_bytes()
        ).expect("logs JSON");
        assert!(logs.iter().any(|item| item.id == log.id));

        let visibility_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("PUT").uri("/api/v1/activity/visibility")
                .header("content-type", "application/json").header("cookie", &cookie)
                .body(Body::from(r#"{"scope":"overall","visibility":"global"}"#))
                .expect("visibility request")
        ).await.expect("visibility response");
        assert_eq!(visibility_response.status(), 200);
        let visibility: ActivityVisibility = serde_json::from_slice(
            &visibility_response.into_body().collect().await.expect("visibility body").to_bytes()
        ).expect("visibility JSON");
        assert_eq!(visibility.scope, "overall");
        assert_eq!(visibility.visibility, "global");

        let policies_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/activity/visibility").header("cookie", &cookie)
                .body(Body::empty()).expect("policies request")
        ).await.expect("policies response");
        assert_eq!(policies_response.status(), 200);
        let policies: Vec<ActivityVisibility> = serde_json::from_slice(
            &policies_response.into_body().collect().await.expect("policies body").to_bytes()
        ).expect("policies JSON");
        assert!(policies.iter().any(|policy| policy.scope == "overall" && policy.visibility == "global"));

        let summary_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/activity/summary?scope=overall&period_days=30")
                .header("cookie", &cookie).body(Body::empty()).expect("summary request")
        ).await.expect("summary response");
        assert_eq!(summary_response.status(), 200);
        let summary: ActivitySummary = serde_json::from_slice(
            &summary_response.into_body().collect().await.expect("summary body").to_bytes()
        ).expect("summary JSON");
        assert_eq!(summary.qso_count, 1);

        let invalid_summary = router_with_store(store, false).oneshot(
            Request::builder().uri("/api/v1/activity/summary?scope=invalid")
                .header("cookie", cookie).body(Body::empty()).expect("invalid summary request")
        ).await.expect("invalid summary response");
        assert_eq!(invalid_summary.status(), 400);
    }

    #[tokio::test]
    async fn member_can_register_reissue_and_revoke_a_session_device() {
        let Ok(database_url) = std::env::var("QSONAUT_TEST_DATABASE_URL") else {
            eprintln!("QSONAUT_TEST_DATABASE_URL is unset; skipping device lifecycle contract");
            return;
        };
        let store = Store::connect(&database_url).await.expect("connect test database");
        let user_id = Uuid::new_v4();
        let callsign = format!("D{}", &user_id.simple().to_string()[..8]).to_ascii_uppercase();
        let password = "device-lifecycle-contract-password";
        let hash = auth::hash_password(password.to_owned()).await.expect("hash password");
        sqlx::query("INSERT INTO users (id,callsign,display_name,password_hash,global_role) VALUES ($1,$2,'Device member',$3,'member')")
            .bind(user_id).bind(&callsign).bind(hash).execute(store.pool()).await
            .expect("insert device test user");
        let login = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"callsign":"{callsign}","password":"{password}"}}"#)))
                .expect("login request")
        ).await.expect("login response");
        assert_eq!(login.status(), 200);
        let cookie = login.headers().get("set-cookie").expect("session cookie")
            .to_str().expect("cookie text").split(';').next().expect("cookie value").to_owned();

        let register = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST").uri("/api/v1/auth/device/session")
                .header("content-type", "application/json").header("cookie", &cookie)
                .body(Body::from(r#"{"device_name":"field laptop"}"#))
                .expect("device registration request")
        ).await.expect("device registration response");
        assert_eq!(register.status(), 200);
        let registered: DeviceToken = serde_json::from_slice(
            &register.into_body().collect().await.expect("device body").to_bytes()
        ).expect("device JSON");
        assert!(!registered.token.is_empty());
        assert!(registered.scopes.iter().any(|scope| scope == "logs:write"));

        let devices_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/auth/devices").header("cookie", &cookie)
                .body(Body::empty()).expect("devices request")
        ).await.expect("devices response");
        assert_eq!(devices_response.status(), 200);
        let devices: Vec<DeviceTokenRecord> = serde_json::from_slice(
            &devices_response.into_body().collect().await.expect("devices body").to_bytes()
        ).expect("devices JSON");
        let device = devices.iter().find(|device| device.device_name == "field laptop").expect("registered device");
        let device_id = device.id;

        let reissue = router_with_store(store.clone(), false).oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/v1/auth/devices/{device_id}/reissue"))
                .header("cookie", &cookie).body(Body::empty()).expect("reissue request")
        ).await.expect("reissue response");
        assert_eq!(reissue.status(), 200);
        let replacement: DeviceToken = serde_json::from_slice(
            &reissue.into_body().collect().await.expect("replacement body").to_bytes()
        ).expect("replacement JSON");
        assert_ne!(replacement.token, registered.token);

        let remaining_response = router_with_store(store.clone(), false).oneshot(
            Request::builder().uri("/api/v1/auth/devices").header("cookie", &cookie)
                .body(Body::empty()).expect("remaining devices request")
        ).await.expect("remaining devices response");
        let remaining: Vec<DeviceTokenRecord> = serde_json::from_slice(
            &remaining_response.into_body().collect().await.expect("remaining devices body").to_bytes()
        ).expect("remaining devices JSON");
        let replacement_id = remaining.iter().find(|device| device.device_name == "field laptop").expect("replacement device").id;
        assert_ne!(replacement_id, device_id);

        let revoke = router_with_store(store, false).oneshot(
            Request::builder().method("DELETE")
                .uri(format!("/api/v1/auth/devices/{replacement_id}"))
                .header("cookie", cookie).body(Body::empty()).expect("revoke request")
        ).await.expect("revoke response");
        assert_eq!(revoke.status(), 204);
    }

    #[test]
    fn openapi_covers_management_and_authentication_routes() {
        let document = ApiDoc::openapi();
        let paths = document.paths.paths;
        for path in [
            "/api/v1/auth/setup",
            "/api/v1/auth/login",
            "/api/v1/access/challenge",
            "/api/v1/access/lookup/{callsign}",
            "/api/v1/access/request",
            "/api/v1/access/requests",
            "/api/v1/access/requests/{request_id}",
            "/api/v1/auth/device",
            "/api/v1/auth/device/session",
            "/api/v1/auth/devices",
            "/api/v1/auth/devices/{token_id}",
            "/api/v1/auth/devices/{token_id}/reissue",
            "/api/v1/clubs",
            "/api/v1/clubs/{club_id}",
            "/api/v1/members",
            "/api/v1/members/{member_id}",
            "/api/v1/clubs/{club_id}/members/{member_id}",
            "/api/v1/clubs/{club_id}/join-requests",
            "/api/v1/clubs/{club_id}/join-requests/{request_id}",
            "/api/v1/contest-templates",
            "/api/v1/events",
            "/api/v1/events/{event_id}/status",
            "/api/v1/events/{event_id}",
            "/api/v1/stations",
            "/api/v1/channel-messages",
            "/api/v1/stations/presence",
            "/api/v1/logs",
            "/api/v1/logs/{log_id}/share",
            "/api/v1/share/{token}",
            "/api/v1/shares",
            "/api/v1/shares/{share_id}",
            "/api/v1/activity/summary",
            "/api/v1/activity/visibility",
            "/api/v1/diagnostics",
            "/api/v1/diagnostics/export",
            "/api/v1/diagnostics/retention/purge",
        ] {
            assert!(paths.contains_key(path), "OpenAPI is missing {path}");
        }
    }
}
