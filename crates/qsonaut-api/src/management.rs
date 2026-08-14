use crate::{
    AppState,
    auth::{
        hash_password, normalize_call, require_admin, require_user, validate_callsign,
        validate_display_name, validate_identity, validate_password,
    },
    error::{HttpError, HttpResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use qsonaut_protocol::{
    ChannelMessage, Club, ClubElection, ClubElectionInput, ClubElectionStatusInput, ClubGovernance,
    ClubInput, ClubJoinDecisionInput, ClubJoinRequest, ClubMembership, ClubMembershipInput,
    ClubPosition, ClubPositionAssignment, ClubPositionAssignmentInput, ClubPositionInput,
    ContestTemplate, CurrentUser, DiagnosticReport, Event, EventInput, EventStatusInput,
    MemberDetail, MemberInput, MemberUpdateInput, PasswordResetInput, QsoLog, QsoLogInput,
    StationPresence, StationPresenceInput,
};
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/members", tag = "management", responses((status = 200, body = [CurrentUser]), (status = 403, description = "Administrator required")))]
pub(crate) async fn members(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<CurrentUser>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.users().await?))
}
#[utoipa::path(post, path = "/api/v1/members", tag = "management", request_body = MemberInput, responses((status = 200, body = CurrentUser)))]
pub(crate) async fn create_member(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<MemberInput>,
) -> HttpResult<Json<CurrentUser>> {
    require_admin(&state, &jar).await?;
    validate_identity(&input.callsign, &input.display_name, &input.password)?;
    let hash = hash_password(input.password).await?;
    Ok(Json(
        state
            .store
            .create_member(
                &normalize_call(&input.callsign),
                input.display_name.trim(),
                &hash,
            )
            .await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/members/{member_id}", tag = "management", params(("member_id" = Uuid, Path)), responses((status = 200, body = MemberDetail), (status = 404, description = "Member not found")))]
pub(crate) async fn member_detail(
    Path(member_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<MemberDetail>> {
    require_admin(&state, &jar).await?;
    let user = state
        .store
        .user(member_id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    let (memberships, stations) = tokio::try_join!(
        state.store.member_memberships(member_id),
        state.store.station_presence(Some(member_id))
    )?;
    Ok(Json(MemberDetail {
        user,
        memberships,
        stations,
    }))
}
#[utoipa::path(patch, path = "/api/v1/members/{member_id}", tag = "management", params(("member_id" = Uuid, Path)), request_body = MemberUpdateInput, responses((status = 200, body = CurrentUser)))]
pub(crate) async fn update_member(
    Path(member_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<MemberUpdateInput>,
) -> HttpResult<Json<CurrentUser>> {
    require_admin(&state, &jar).await?;
    validate_display_name(&input.display_name)?;
    state
        .store
        .update_member_display_name(member_id, input.display_name.trim())
        .await?
        .map(Json)
        .ok_or_else(HttpError::not_found)
}
#[utoipa::path(post, path = "/api/v1/members/{member_id}/password", tag = "management", params(("member_id" = Uuid, Path)), request_body = PasswordResetInput, responses((status = 204, description = "Password changed and existing sessions revoked")))]
pub(crate) async fn reset_member_password(
    Path(member_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<PasswordResetInput>,
) -> HttpResult<StatusCode> {
    require_admin(&state, &jar).await?;
    validate_password(&input.password)?;
    let hash = hash_password(input.password).await?;
    if !state
        .store
        .update_member_password_hash(member_id, &hash)
        .await?
    {
        return Err(HttpError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path = "/api/v1/clubs/{club_id}/members", tag = "management", params(("club_id" = Uuid, Path)), responses((status = 200, body = [ClubMembership])))]
pub(crate) async fn club_members(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ClubMembership>>> {
    let user = require_user(&state, &jar).await?;
    require_club_manager(&state, &user, club_id).await?;
    Ok(Json(state.store.club_members(club_id).await?))
}
#[utoipa::path(put, path = "/api/v1/clubs/{club_id}/members", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubMembershipInput, responses((status = 200, body = ClubMembership)))]
pub(crate) async fn set_club_member(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubMembershipInput>,
) -> HttpResult<Json<ClubMembership>> {
    let user = require_user(&state, &jar).await?;
    let authority = require_club_manager(&state, &user, club_id).await?;
    if !["owner", "coordinator", "operator", "observer"].contains(&input.role.as_str()) {
        return Err(HttpError::bad_request("invalid club role"));
    }
    if input
        .membership_status
        .as_deref()
        .is_some_and(|status| !["active", "lapsed", "inactive"].contains(&status))
    {
        return Err(HttpError::bad_request("invalid membership status"));
    }
    if input.dues_status.as_deref().is_some_and(|status| {
        !["not_tracked", "current", "due", "overdue", "waived"].contains(&status)
    }) {
        return Err(HttpError::bad_request("invalid dues status"));
    }
    let target_role = state.store.club_role(club_id, input.user_id).await?;
    enforce_club_role_assignment(&authority, target_role.as_deref(), &input.role)?;
    if target_role.as_deref() == Some("owner")
        && (input.role != "owner"
            || input
                .membership_status
                .as_deref()
                .is_some_and(|status| status != "active"))
        && state.store.active_owner_count(club_id).await? <= 1
    {
        return Err(HttpError::conflict(
            "a club must keep at least one active owner",
        ));
    }
    Ok(Json(
        state
            .store
            .set_club_member(club_id, input.user_id, &input)
            .await?,
    ))
}
#[utoipa::path(delete, path = "/api/v1/clubs/{club_id}/members/{member_id}", tag = "management", params(("club_id" = Uuid, Path), ("member_id" = Uuid, Path)), responses((status = 204, description = "Membership removed"), (status = 404, description = "Membership not found")))]
pub(crate) async fn remove_club_member(
    Path((club_id, member_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<StatusCode> {
    let user = require_user(&state, &jar).await?;
    let authority = require_club_manager(&state, &user, club_id).await?;
    let target_role = state
        .store
        .club_role(club_id, member_id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    enforce_club_role_assignment(&authority, Some(&target_role), "operator")?;
    if target_role == "owner" && state.store.active_owner_count(club_id).await? <= 1 {
        return Err(HttpError::conflict(
            "a club must keep at least one active owner",
        ));
    }
    if !state.store.remove_club_member(club_id, member_id).await? {
        return Err(HttpError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/clubs", tag = "management", responses((status = 200, body = [Club])))]
pub(crate) async fn clubs(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<Club>>> {
    let user = require_user(&state, &jar).await?;
    Ok(Json(
        state
            .store
            .clubs(user.id, user.global_role == "administrator")
            .await?,
    ))
}
#[utoipa::path(post, path = "/api/v1/clubs", tag = "management", request_body = ClubInput, responses((status = 200, body = Club)))]
pub(crate) async fn create_club(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(mut input): Json<ClubInput>,
) -> HttpResult<Json<Club>> {
    let user = require_user(&state, &jar).await?;
    if input.name.trim().is_empty() {
        return Err(HttpError::bad_request("club name is required"));
    }
    input.callsign = input
        .callsign
        .map(|call| call.trim().to_ascii_uppercase())
        .filter(|call| !call.is_empty());
    Ok(Json(state.store.create_club(&input, user.id).await?))
}

#[utoipa::path(post, path = "/api/v1/clubs/{club_id}/join-requests", tag = "management", params(("club_id" = Uuid, Path)), responses((status = 200, body = ClubJoinRequest)))]
pub(crate) async fn request_club_join(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<ClubJoinRequest>> {
    let user = require_user(&state, &jar).await?;
    if state.store.club_role(club_id, user.id).await?.is_some() {
        return Err(HttpError::conflict("you are already a member of this club"));
    }
    Ok(Json(state.store.request_club_join(club_id, user.id).await?))
}

#[utoipa::path(get, path = "/api/v1/clubs/{club_id}/join-requests", tag = "management", params(("club_id" = Uuid, Path)), responses((status = 200, body = [ClubJoinRequest])))]
pub(crate) async fn club_join_requests(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ClubJoinRequest>>> {
    let user = require_user(&state, &jar).await?;
    require_club_manager(&state, &user, club_id).await?;
    Ok(Json(state.store.club_join_requests(club_id).await?))
}

#[utoipa::path(patch, path = "/api/v1/clubs/{club_id}/join-requests/{request_id}", tag = "management", params(("club_id" = Uuid, Path), ("request_id" = Uuid, Path)), request_body = ClubJoinDecisionInput, responses((status = 200, body = ClubJoinRequest)))]
pub(crate) async fn review_club_join_request(
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubJoinDecisionInput>,
) -> HttpResult<Json<ClubJoinRequest>> {
    let user = require_user(&state, &jar).await?;
    let authority = require_club_manager(&state, &user, club_id).await?;
    if !["approve", "reject"].contains(&input.decision.as_str()) {
        return Err(HttpError::bad_request("decision must be approve or reject"));
    }
    if !["coordinator", "operator", "observer"].contains(&input.role.as_str()) {
        return Err(HttpError::bad_request("invalid approved club role"));
    }
    enforce_club_role_assignment(&authority, None, &input.role)?;
    state
        .store
        .review_club_join_request(
            club_id,
            request_id,
            user.id,
            if input.decision == "approve" {
                "approved"
            } else {
                "rejected"
            },
            &input.role,
        )
        .await?
        .map(Json)
        .ok_or_else(|| HttpError::conflict("join request is no longer pending"))
}

#[utoipa::path(get, path = "/api/v1/clubs/{club_id}/governance", tag = "management", params(("club_id" = Uuid, Path)), responses((status = 200, body = ClubGovernance)))]
pub(crate) async fn club_governance(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<ClubGovernance>> {
    require_user(&state, &jar).await?;
    Ok(Json(state.store.club_governance(club_id).await?))
}

#[utoipa::path(post, path = "/api/v1/clubs/{club_id}/positions", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubPositionInput, responses((status = 200, body = ClubPosition)))]
pub(crate) async fn create_club_position(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubPositionInput>,
) -> HttpResult<Json<ClubPosition>> {
    let user = require_user(&state, &jar).await?;
    require_club_owner(&state, &user, club_id).await?;
    if input.name.trim().is_empty() {
        return Err(HttpError::bad_request("position name is required"));
    }
    if !["officer", "board"].contains(&input.position_type.as_str())
        || !["any", "even", "odd"].contains(&input.election_parity.as_str())
        || !(1..=100).contains(&input.seats)
        || !(1..=10).contains(&input.term_years)
    {
        return Err(HttpError::bad_request("invalid position configuration"));
    }
    Ok(Json(
        state.store.create_club_position(club_id, &input).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/clubs/{club_id}/position-assignments", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubPositionAssignmentInput, responses((status = 200, body = ClubPositionAssignment)))]
pub(crate) async fn assign_club_position(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubPositionAssignmentInput>,
) -> HttpResult<Json<ClubPositionAssignment>> {
    let user = require_user(&state, &jar).await?;
    require_club_owner(&state, &user, club_id).await?;
    if input.ends_on < input.starts_on
        || input.seat_number < 1
        || !["elected", "appointed", "acting"].contains(&input.selection_method.as_str())
    {
        return Err(HttpError::bad_request("invalid position assignment"));
    }
    Ok(Json(
        state.store.assign_club_position(club_id, &input).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/clubs/{club_id}/elections", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubElectionInput, responses((status = 200, body = ClubElection)))]
pub(crate) async fn create_club_election(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubElectionInput>,
) -> HttpResult<Json<ClubElection>> {
    let user = require_user(&state, &jar).await?;
    require_club_owner(&state, &user, club_id).await?;
    if input.title.trim().is_empty()
        || !(2000..=2200).contains(&input.election_year)
        || ![
            "planned",
            "nominations",
            "voting",
            "closed",
            "certified",
            "cancelled",
        ]
        .contains(&input.status.as_str())
        || input
            .opens_at
            .zip(input.closes_at)
            .is_some_and(|(opens, closes)| closes <= opens)
    {
        return Err(HttpError::bad_request("invalid election configuration"));
    }
    let governance = state.store.club_governance(club_id).await?;
    for position_id in &input.position_ids {
        let position = governance
            .positions
            .iter()
            .find(|position| position.id == *position_id)
            .ok_or_else(|| HttpError::bad_request("election contains an unknown position"))?;
        let year_parity = if input.election_year % 2 == 0 {
            "even"
        } else {
            "odd"
        };
        if position.election_parity != "any" && position.election_parity != year_parity {
            return Err(HttpError::bad_request(format!(
                "{} is configured for {}-year elections",
                position.name, position.election_parity
            )));
        }
    }
    Ok(Json(
        state
            .store
            .create_club_election(club_id, user.id, &input)
            .await?,
    ))
}

#[utoipa::path(patch, path = "/api/v1/clubs/{club_id}/elections/{election_id}", tag = "management", params(("club_id" = Uuid, Path), ("election_id" = Uuid, Path)), request_body = ClubElectionStatusInput, responses((status = 200, body = ClubElection)))]
pub(crate) async fn set_club_election_status(
    Path((club_id, election_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubElectionStatusInput>,
) -> HttpResult<Json<ClubElection>> {
    let user = require_user(&state, &jar).await?;
    require_club_owner(&state, &user, club_id).await?;
    if ![
        "planned",
        "nominations",
        "voting",
        "closed",
        "certified",
        "cancelled",
    ]
    .contains(&input.status.as_str())
    {
        return Err(HttpError::bad_request("invalid election status"));
    }
    state
        .store
        .set_club_election_status(club_id, election_id, &input.status)
        .await?
        .map(Json)
        .ok_or_else(HttpError::not_found)
}
#[utoipa::path(get, path = "/api/v1/contest-templates", tag = "management", responses((status = 200, body = [ContestTemplate])))]
pub(crate) async fn contest_templates(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ContestTemplate>>> {
    require_user(&state, &jar).await?;
    Ok(Json(state.store.contest_templates().await?))
}
#[utoipa::path(get, path = "/api/v1/events", tag = "management", responses((status = 200, body = [Event])))]
pub(crate) async fn events(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<Event>>> {
    require_user(&state, &jar).await?;
    Ok(Json(state.store.events().await?))
}
#[utoipa::path(post, path = "/api/v1/events", tag = "management", request_body = EventInput, responses((status = 200, body = Event)))]
pub(crate) async fn create_event(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(mut input): Json<EventInput>,
) -> HttpResult<Json<Event>> {
    require_admin(&state, &jar).await?;
    if input.name.trim().is_empty() {
        return Err(HttpError::bad_request("event name is required"));
    }
    if input.ends_at <= input.starts_at {
        return Err(HttpError::bad_request("event end must be after its start"));
    }
    if let Some(template_id) = input.contest_template_id {
        let template = state
            .store
            .contest_templates()
            .await?
            .into_iter()
            .find(|template| template.id == template_id)
            .ok_or_else(|| HttpError::bad_request("unknown contest template"))?;
        validate_contest_configuration(&template, &input.contest_config)?;
        input.contest_name = template.name;
    } else {
        input.contest_name.clear();
        input.contest_config = serde_json::json!({});
    }
    input.special_callsign = input
        .special_callsign
        .map(|call| call.trim().to_ascii_uppercase())
        .filter(|call| !call.is_empty());
    Ok(Json(state.store.create_event(&input).await?))
}

#[utoipa::path(patch, path = "/api/v1/events/{event_id}/status", tag = "management", params(("event_id" = Uuid, Path)), request_body = EventStatusInput, responses((status = 200, body = Event), (status = 404, description = "Event not found")))]
pub(crate) async fn set_event_status(
    Path(event_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<EventStatusInput>,
) -> HttpResult<Json<Event>> {
    require_admin(&state, &jar).await?;
    state
        .store
        .set_event_status(event_id, input.status)
        .await?
        .map(Json)
        .ok_or_else(HttpError::not_found)
}

#[utoipa::path(get, path = "/api/v1/stations", tag = "activity", responses((status = 200, body = [StationPresence])))]
pub(crate) async fn stations(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<StationPresence>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.station_presence(None).await?))
}

#[utoipa::path(put, path = "/api/v1/stations/presence", tag = "activity", request_body = StationPresenceInput, responses((status = 200, body = StationPresence)))]
pub(crate) async fn publish_station_presence(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<StationPresenceInput>,
) -> HttpResult<Json<StationPresence>> {
    let user = require_user(&state, &jar).await?;
    if !["online", "idle", "offline"].contains(&input.status.as_str()) {
        return Err(HttpError::bad_request("invalid station status"));
    }
    if input.qsonaut_version.trim().is_empty() || input.platform.trim().is_empty() {
        return Err(HttpError::bad_request(
            "QSONaut version and platform are required",
        ));
    }
    if input.frequency_hz.is_some_and(|frequency| frequency < 0) {
        return Err(HttpError::bad_request("frequency cannot be negative"));
    }
    if !input.metadata.is_object() || input.metadata.to_string().len() > 8_192 {
        return Err(HttpError::bad_request(
            "station metadata must be an object no larger than 8 KiB",
        ));
    }
    Ok(Json(
        state.store.upsert_station_presence(user.id, &input).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/logs", tag = "activity", responses((status = 200, body = [QsoLog])))]
pub(crate) async fn logs(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<QsoLog>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.qso_logs(500).await?))
}

#[utoipa::path(get, path = "/api/v1/diagnostics", tag = "activity", responses((status = 200, body = [DiagnosticReport])))]
pub(crate) async fn diagnostics(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<qsonaut_protocol::DiagnosticReport>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.diagnostic_reports(500).await?))
}

#[utoipa::path(get, path = "/api/v1/channel-messages", tag = "activity", responses((status = 200, body = [ChannelMessage])))]
pub(crate) async fn channel_messages(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ChannelMessage>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.channel_messages(200).await?))
}

#[utoipa::path(post, path = "/api/v1/logs", tag = "activity", request_body = QsoLogInput, responses((status = 200, body = QsoLog)))]
pub(crate) async fn collect_log(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<QsoLogInput>,
) -> HttpResult<Json<QsoLog>> {
    let user = require_user(&state, &jar).await?;
    validate_callsign(&input.callsign)?;
    if input.band.trim().is_empty() || input.mode.trim().is_empty() {
        return Err(HttpError::bad_request(
            "callsign, band, and mode are required",
        ));
    }
    if input.frequency_hz.is_some_and(|frequency| frequency < 0) {
        return Err(HttpError::bad_request("frequency cannot be negative"));
    }
    if !input.exchange.is_object() || input.exchange.to_string().len() > 8_192 {
        return Err(HttpError::bad_request(
            "exchange must be an object no larger than 8 KiB",
        ));
    }
    if input.source.trim().is_empty() || input.source.len() > 40 {
        return Err(HttpError::bad_request(
            "log source must contain 1 to 40 characters",
        ));
    }
    Ok(Json(state.store.create_qso_log(user.id, &input).await?))
}

fn validate_contest_configuration(
    template: &ContestTemplate,
    contest_config: &serde_json::Value,
) -> HttpResult<()> {
    let config = contest_config
        .as_object()
        .ok_or_else(|| HttpError::bad_request("contest configuration must be an object"))?;
    if let Some(fields) = template.required_fields.as_object() {
        for (name, field) in fields {
            let required = field
                .get("required")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let value = config
                .get(name)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if required && value.is_empty() {
                return Err(HttpError::bad_request(format!(
                    "contest field {name} is required"
                )));
            }
            if !value.is_empty()
                && let Some(options) = field.get("options").and_then(serde_json::Value::as_array)
                && !options.iter().any(|option| option.as_str() == Some(value))
            {
                return Err(HttpError::bad_request(format!(
                    "invalid value for contest field {name}"
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClubAuthority {
    is_administrator: bool,
    role: Option<String>,
}

async fn require_club_manager(
    state: &AppState,
    user: &CurrentUser,
    club_id: Uuid,
) -> HttpResult<ClubAuthority> {
    let is_administrator = user.global_role == "administrator";
    let role = state.store.club_role(club_id, user.id).await?;
    if !is_administrator
        && !role
            .as_deref()
            .is_some_and(|role| matches!(role, "owner" | "coordinator"))
    {
        return Err(HttpError::forbidden_with(
            "club owner or coordinator access required",
        ));
    }
    Ok(ClubAuthority {
        is_administrator,
        role,
    })
}

async fn require_club_owner(state: &AppState, user: &CurrentUser, club_id: Uuid) -> HttpResult<()> {
    if user.global_role == "administrator"
        || state.store.club_role(club_id, user.id).await?.as_deref() == Some("owner")
    {
        return Ok(());
    }
    Err(HttpError::forbidden_with(
        "club owner access required for governance changes",
    ))
}

fn enforce_club_role_assignment(
    authority: &ClubAuthority,
    target_role: Option<&str>,
    assigned_role: &str,
) -> HttpResult<()> {
    if authority.is_administrator || authority.role.as_deref() == Some("owner") {
        return Ok(());
    }
    if target_role.is_some_and(|role| matches!(role, "owner" | "coordinator"))
        || !matches!(assigned_role, "operator" | "observer")
    {
        return Err(HttpError::forbidden_with(
            "coordinators may manage operator and observer memberships",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinators_cannot_promote_or_manage_club_leadership() {
        let coordinator = ClubAuthority {
            is_administrator: false,
            role: Some("coordinator".to_owned()),
        };
        assert!(enforce_club_role_assignment(&coordinator, Some("operator"), "observer").is_ok());
        assert!(enforce_club_role_assignment(&coordinator, None, "coordinator").is_err());
        assert!(enforce_club_role_assignment(&coordinator, Some("owner"), "operator").is_err());
    }

    #[test]
    fn owners_and_global_administrators_can_assign_leadership() {
        let owner = ClubAuthority {
            is_administrator: false,
            role: Some("owner".to_owned()),
        };
        let administrator = ClubAuthority {
            is_administrator: true,
            role: None,
        };
        assert!(enforce_club_role_assignment(&owner, None, "coordinator").is_ok());
        assert!(enforce_club_role_assignment(&administrator, Some("owner"), "owner").is_ok());
    }

    fn template() -> ContestTemplate {
        ContestTemplate {
            id: Uuid::nil(),
            contest_type: "TEST".to_owned(),
            name: "Test contest".to_owned(),
            description: String::new(),
            organization: "QSONaut".to_owned(),
            icon: "🏁".to_owned(),
            rules_url: "https://example.com/rules".to_owned(),
            definition_version: 1,
            is_builtin: false,
            scoring_rules: serde_json::json!({}),
            required_fields: serde_json::json!({
                "class": { "required": true, "options": ["1A", "2A"] },
                "section": { "required": true }
            }),
            validation_rules: serde_json::json!({}),
            schedule: serde_json::json!({}),
        }
    }

    #[test]
    fn contest_configuration_requires_fields() {
        assert!(validate_contest_configuration(&template(), &serde_json::json!({})).is_err());
    }

    #[test]
    fn contest_configuration_rejects_unknown_option() {
        assert!(
            validate_contest_configuration(
                &template(),
                &serde_json::json!({ "class": "9Z", "section": "OR" })
            )
            .is_err()
        );
    }

    #[test]
    fn contest_configuration_accepts_complete_values() {
        assert!(
            validate_contest_configuration(
                &template(),
                &serde_json::json!({ "class": "2A", "section": "OR" })
            )
            .is_ok()
        );
    }
}
