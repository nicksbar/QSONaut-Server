//! Versioned HTTP API assembly for `QSONaut` Server.

mod auth;
mod error;
mod management;
mod realtime;

use axum::{
    Json, Router,
    routing::{get, patch, post},
};
use qsonaut_protocol::{
    API_VERSION, ApiError, BootstrapRequest, ChannelMessage, Club, ClubElection, ClubElectionInput,
    ClubElectionStatusInput, ClubGovernance, ClubInput, ClubJoinDecisionInput, ClubJoinRequest,
    ClubMembership, ClubMembershipInput, ClubPosition, ClubPositionAssignment,
    ClubPositionAssignmentInput, ClubPositionInput, ContestTemplate, Credentials, CurrentUser,
    DeviceCredentials, DeviceRegistration, DeviceToken, DeviceTokenRecord, DiagnosticReport,
    DiagnosticReportInput, Event, EventInput, EventStatus, EventStatusInput, HealthResponse,
    MemberClubRole, MemberDetail, MemberInput, MemberUpdateInput, PasswordResetInput, QsoLog,
    QsoLogInput, ServiceInfo, ServiceStatus, SetupStatus, StationPresence, StationPresenceInput,
};
use qsonaut_store::Store;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health, service_info,
        auth::setup_status, auth::bootstrap, auth::login, auth::me, auth::logout,
        auth::device_login, auth::register_session_device, auth::session_devices,
        auth::reissue_session_device, auth::revoke_session_device, auth::revoke_device,
        management::members, management::create_member,
        management::member_detail, management::update_member, management::reset_member_password,
        management::club_members, management::set_club_member,
        management::remove_club_member,
        management::request_club_join, management::club_join_requests,
        management::review_club_join_request,
        management::club_governance, management::create_club_position,
        management::assign_club_position, management::create_club_election,
        management::set_club_election_status,
        management::clubs, management::create_club,
        management::contest_templates,
        management::events, management::create_event, management::set_event_status,
        management::stations, management::publish_station_presence, management::channel_messages,
        management::logs, management::collect_log, management::diagnostics
    ),
    components(schemas(
        HealthResponse, ServiceInfo, ServiceStatus, ApiError, SetupStatus, Credentials,
        DeviceCredentials, DeviceRegistration, DeviceToken, DeviceTokenRecord,
        BootstrapRequest, CurrentUser, MemberInput, ClubMembership, ClubMembershipInput,
        ClubJoinRequest, ClubJoinDecisionInput,
        ClubGovernance, ClubPosition, ClubPositionInput,
        ClubPositionAssignment, ClubPositionAssignmentInput, ClubElection, ClubElectionInput,
        ClubElectionStatusInput,
        MemberUpdateInput, PasswordResetInput, MemberClubRole, MemberDetail,
        Club, ClubInput, ContestTemplate, EventStatus, Event, EventInput, EventStatusInput, ChannelMessage,
        StationPresence, StationPresenceInput, QsoLog, QsoLogInput, DiagnosticReport, DiagnosticReportInput
    )),
    tags(
        (name = "service", description = "Service discovery and readiness"),
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
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/info", get(service_info))
        .route("/api/v1/openapi.json", get(openapi))
}

pub fn router_with_store(store: Store, secure_cookies: bool) -> Router {
    let (channel_messages, _) = tokio::sync::broadcast::channel(256);
    let state = AppState {
        store,
        secure_cookies,
        channel_messages,
    };
    let management = Router::<AppState>::new()
        .route(
            "/api/v1/auth/setup",
            get(auth::setup_status).post(auth::bootstrap),
        )
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
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
        .route(
            "/api/v1/events",
            get(management::events).post(management::create_event),
        )
        .route(
            "/api/v1/events/{event_id}/status",
            patch(management::set_event_status),
        )
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
            "/api/v1/clubs/{club_id}/join-requests",
            get(management::club_join_requests).post(management::request_club_join),
        )
        .route(
            "/api/v1/clubs/{club_id}/join-requests/{request_id}",
            patch(management::review_club_join_request),
        )
        .route(
            "/api/v1/clubs/{club_id}/governance",
            get(management::club_governance),
        )
        .route(
            "/api/v1/clubs/{club_id}/positions",
            post(management::create_club_position),
        )
        .route(
            "/api/v1/clubs/{club_id}/position-assignments",
            post(management::assign_club_position),
        )
        .route(
            "/api/v1/clubs/{club_id}/elections",
            post(management::create_club_election),
        )
        .route(
            "/api/v1/clubs/{club_id}/elections/{election_id}",
            patch(management::set_club_election_status),
        )
        .route(
            "/api/v1/contest-templates",
            get(management::contest_templates),
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
            "clubs and membership".to_owned(),
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
            "/api/v1/auth/device",
            "/api/v1/auth/device/session",
            "/api/v1/auth/devices",
            "/api/v1/auth/devices/{token_id}",
            "/api/v1/auth/devices/{token_id}/reissue",
            "/api/v1/clubs",
            "/api/v1/members",
            "/api/v1/members/{member_id}",
            "/api/v1/clubs/{club_id}/members/{member_id}",
            "/api/v1/clubs/{club_id}/join-requests",
            "/api/v1/clubs/{club_id}/join-requests/{request_id}",
            "/api/v1/clubs/{club_id}/governance",
            "/api/v1/clubs/{club_id}/positions",
            "/api/v1/clubs/{club_id}/position-assignments",
            "/api/v1/clubs/{club_id}/elections",
            "/api/v1/clubs/{club_id}/elections/{election_id}",
            "/api/v1/contest-templates",
            "/api/v1/events",
            "/api/v1/events/{event_id}/status",
            "/api/v1/stations",
            "/api/v1/channel-messages",
            "/api/v1/stations/presence",
            "/api/v1/logs",
            "/api/v1/diagnostics",
        ] {
            assert!(paths.contains_key(path), "OpenAPI is missing {path}");
        }
    }
}
