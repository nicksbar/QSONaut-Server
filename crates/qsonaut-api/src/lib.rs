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
        management::event_participants, management::create_event_participant, management::event_score,
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
    use http_body_util::BodyExt;
    use tower::ServiceExt;

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
