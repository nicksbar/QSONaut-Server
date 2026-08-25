use crate::{
    AppState,
    auth::{
        hash_password, normalize_call, require_admin, require_user, token_hash, validate_callsign,
        validate_display_name, validate_identity, validate_password,
    },
    error::{HttpError, HttpResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use qsonaut_protocol::{
    ActivitySummary, ActivityVisibility, ActivityVisibilityInput, ChannelMessage, Club, ClubInput,
    ClubJoinDecisionInput, ClubJoinRequest, ClubMembership, ClubMembershipInput, ContestTemplate,
    CurrentUser, DiagnosticReport, Event, EventInput, EventStatusInput, EventUpdateInput,
    MemberDetail, MemberInput, MemberUpdateInput, PasswordResetInput, QsoLog, QsoLogInput,
    ShareLink, ShareLinkInput, ShareLinkRecord, SharedQsoDetail, StationPresence,
    StationPresenceInput,
};
use rand::RngCore;
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
    let current = state
        .store
        .user(member_id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    let requested_role = input.global_role.as_deref().unwrap_or(&current.global_role);
    if !["administrator", "member"].contains(&requested_role) {
        return Err(HttpError::bad_request("invalid global role"));
    }
    let updated = state
        .store
        .update_member_profile(member_id, input.display_name.trim(), requested_role)
        .await?
        .ok_or_else(|| {
            if current.global_role == "administrator" && requested_role == "member" {
                HttpError::conflict("the server must keep at least one administrator")
            } else {
                HttpError::not_found()
            }
        })?;
    Ok(Json(updated))
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
    let club = state
        .store
        .create_club_with_limit(&input, user.id, state.policy.max_clubs)
        .await?
        .ok_or_else(|| {
            HttpError::conflict(format!(
                "the {} edition supports up to {} clubs; this limit is removed by the hosted extension",
                state.policy.edition,
                state.policy.max_clubs.unwrap_or_default()
            ))
        })?;
    Ok(Json(club))
}

#[utoipa::path(patch, path = "/api/v1/clubs/{club_id}", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubInput, responses((status = 200, body = Club), (status = 404, description = "Club not found or not manageable")))]
pub(crate) async fn update_club(
    Path(club_id): Path<Uuid>,
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
    state
        .store
        .update_club(club_id, user.id, &input)
        .await?
        .map(Json)
        .ok_or_else(HttpError::not_found)
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
    if !state.store.club_exists(input.club_id).await? {
        return Err(HttpError::bad_request("unknown club"));
    }
    validate_event_identity(&input.name, input.special_callsign.as_deref())?;
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

#[utoipa::path(patch, path = "/api/v1/events/{event_id}", tag = "management", params(("event_id" = Uuid, Path)), request_body = EventUpdateInput, responses((status = 200, body = Event), (status = 404, description = "Event not found")))]
pub(crate) async fn update_event(
    Path(event_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(mut input): Json<EventUpdateInput>,
) -> HttpResult<Json<Event>> {
    require_admin(&state, &jar).await?;
    if !state.store.club_exists(input.club_id).await? {
        return Err(HttpError::bad_request("unknown club"));
    }
    validate_event_identity(&input.name, input.special_callsign.as_deref())?;
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
    Ok(Json(
        state
            .store
            .update_event(event_id, &input)
            .await?
            .ok_or_else(HttpError::not_found)?,
    ))
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
    let user = require_user(&state, &jar).await?;
    Ok(Json(
        state
            .store
            .qso_logs_for_viewer(user.id, user.global_role == "administrator", 500)
            .await?,
    ))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub(crate) struct ActivitySummaryQuery {
    #[serde(default = "default_summary_scope")]
    pub scope: String,
    pub scope_id: Option<Uuid>,
    #[serde(default = "default_summary_period")]
    pub period_days: i32,
}

fn default_summary_scope() -> String {
    "overall".to_owned()
}
fn default_summary_period() -> i32 {
    30
}

#[utoipa::path(
    get,
    path = "/api/v1/activity/summary",
    tag = "activity",
    params(ActivitySummaryQuery),
    responses((status = 200, body = ActivitySummary), (status = 400))
)]
pub(crate) async fn activity_summary(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ActivitySummaryQuery>,
) -> HttpResult<Json<ActivitySummary>> {
    let user = require_user(&state, &jar).await?;
    if !["overall", "club", "contest"].contains(&query.scope.as_str()) {
        return Err(HttpError::bad_request("invalid activity summary scope"));
    }
    if !(0..=3650).contains(&query.period_days) {
        return Err(HttpError::bad_request(
            "activity period must be between 0 and 3650 days",
        ));
    }
    if query.scope != "overall" && query.scope_id.is_none() {
        return Err(HttpError::bad_request("this activity scope requires an id"));
    }
    if query.scope == "club" {
        let club_id = query.scope_id.expect("validated above");
        if user.global_role != "administrator"
            && !state.store.active_club_member(user.id, club_id).await?
        {
            return Err(HttpError::forbidden());
        }
    }
    if query.scope == "contest" {
        let event_id = query.scope_id.expect("validated above");
        let club_id = state
            .store
            .event_club_id(event_id)
            .await?
            .ok_or_else(HttpError::not_found)?;
        if user.global_role != "administrator"
            && !state.store.active_club_member(user.id, club_id).await?
        {
            return Err(HttpError::forbidden());
        }
    }
    Ok(Json(
        state
            .store
            .activity_summary(user.id, &query.scope, query.scope_id, query.period_days)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/activity/visibility", tag = "activity", responses((status = 200, body = [ActivityVisibility])))]
pub(crate) async fn activity_visibility(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ActivityVisibility>>> {
    let user = require_user(&state, &jar).await?;
    Ok(Json(
        state.store.activity_visibility_policies(user.id).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/activity/visibility", tag = "activity", request_body = ActivityVisibilityInput, responses((status = 200, body = ActivityVisibility), (status = 400)))]
pub(crate) async fn set_activity_visibility(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ActivityVisibilityInput>,
) -> HttpResult<Json<ActivityVisibility>> {
    let user = require_user(&state, &jar).await?;
    if !["overall", "club", "contest"].contains(&input.scope.as_str()) {
        return Err(HttpError::bad_request("invalid activity visibility scope"));
    }
    if !["private", "members", "global"].contains(&input.visibility.as_str()) {
        return Err(HttpError::bad_request("invalid activity visibility"));
    }
    match input.scope.as_str() {
        "overall" if input.scope_id.is_some() => {
            return Err(HttpError::bad_request(
                "overall visibility cannot have a target",
            ));
        }
        "club" => {
            let club_id = input
                .scope_id
                .ok_or_else(|| HttpError::bad_request("club visibility requires a club"))?;
            if !state.store.active_club_member(user.id, club_id).await? {
                return Err(HttpError::forbidden());
            }
        }
        "contest" => {
            let event_id = input
                .scope_id
                .ok_or_else(|| HttpError::bad_request("contest visibility requires a contest"))?;
            let club_id = state
                .store
                .event_club_id(event_id)
                .await?
                .ok_or_else(HttpError::not_found)?;
            if !state.store.active_club_member(user.id, club_id).await? {
                return Err(HttpError::forbidden());
            }
        }
        _ => {}
    }
    Ok(Json(
        state.store.set_activity_visibility(user.id, &input).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/diagnostics", tag = "activity", responses((status = 200, body = [DiagnosticReport])))]
pub(crate) async fn diagnostics(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<qsonaut_protocol::DiagnosticReport>>> {
    let user = require_admin(&state, &jar).await?;
    state
        .store
        .record_audit_event(
            Some(user.id),
            "diagnostics_inspected",
            "diagnostic_reports",
            None,
            &serde_json::json!({"limit": 500}),
        )
        .await?;
    Ok(Json(state.store.diagnostic_reports(500).await?))
}

#[utoipa::path(get, path = "/api/v1/diagnostics/export", tag = "activity", responses((status = 200, body = [DiagnosticReport]), (status = 403)))]
pub(crate) async fn export_diagnostics(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<qsonaut_protocol::DiagnosticReport>>> {
    let user = require_admin(&state, &jar).await?;
    let reports = state.store.diagnostic_reports(500).await?;
    state
        .store
        .record_audit_event(
            Some(user.id),
            "diagnostics_exported",
            "diagnostic_reports",
            None,
            &serde_json::json!({"count": reports.len(), "limit": 500}),
        )
        .await?;
    Ok(Json(reports))
}

#[utoipa::path(post, path = "/api/v1/diagnostics/retention/purge", tag = "activity", responses((status = 204, description = "Expired diagnostics and share artifacts removed"), (status = 403)))]
pub(crate) async fn purge_retained_artifacts(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<StatusCode> {
    let user = require_admin(&state, &jar).await?;
    state.store.purge_expired_artifacts().await?;
    state
        .store
        .record_audit_event(
            Some(user.id),
            "retention_purge",
            "retention",
            None,
            &serde_json::json!({"diagnostics_days": 30, "share_link_grace_days": 30}),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
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

#[utoipa::path(
    post,
    path = "/api/v1/logs/{log_id}/share",
    tag = "activity",
    params(("log_id" = Uuid, Path)),
    request_body = ShareLinkInput,
    responses((status = 200, body = ShareLink), (status = 404))
)]
pub(crate) async fn create_log_share(
    Path(log_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ShareLinkInput>,
) -> HttpResult<Json<ShareLink>> {
    require_external_sharing(&state)?;
    let user = require_user(&state, &jar).await?;
    let owner = state
        .store
        .qso_log_owner(log_id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    if owner != user.id && user.global_role != "administrator" {
        return Err(HttpError::forbidden());
    }
    if !(1..=30).contains(&input.expires_in_days) {
        return Err(HttpError::bad_request(
            "share expiry must be between 1 and 30 days",
        ));
    }
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let expires_at = Utc::now() + ChronoDuration::days(input.expires_in_days);
    let id = state
        .store
        .create_qso_share_link(log_id, user.id, &token_hash(&token), expires_at)
        .await?;
    state
        .store
        .record_audit_event(
            Some(user.id),
            "share_created",
            "qso_log",
            Some(log_id),
            &serde_json::json!({"expires_at": expires_at}),
        )
        .await?;
    Ok(Json(ShareLink {
        id,
        share_path: format!("/share/{token}"),
        expires_at,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/share/{token}",
    tag = "activity",
    params(("token" = String, Path)),
    responses((status = 200, body = SharedQsoDetail), (status = 404))
)]
pub(crate) async fn shared_log(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> HttpResult<Json<SharedQsoDetail>> {
    require_external_sharing(&state)?;
    let log = state
        .store
        .shared_qso_log(&token_hash(&token))
        .await?
        .ok_or_else(HttpError::not_found)?;
    Ok(Json(log.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/shares/{share_id}",
    tag = "activity",
    params(("share_id" = Uuid, Path)),
    responses((status = 204), (status = 404))
)]
pub(crate) async fn revoke_log_share(
    Path(share_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<StatusCode> {
    require_external_sharing(&state)?;
    let user = require_user(&state, &jar).await?;
    if !state
        .store
        .revoke_qso_share_link(share_id, user.id, user.global_role == "administrator")
        .await?
    {
        return Err(HttpError::not_found());
    }
    state
        .store
        .record_audit_event(
            Some(user.id),
            "share_revoked",
            "share_link",
            Some(share_id),
            &serde_json::json!({}),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/shares", tag = "activity", responses((status = 200, body = [ShareLinkRecord])))]
pub(crate) async fn log_shares(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<ShareLinkRecord>>> {
    require_external_sharing(&state)?;
    let user = require_user(&state, &jar).await?;
    Ok(Json(state.store.qso_share_links_for_user(user.id).await?))
}

fn require_external_sharing(state: &AppState) -> HttpResult<()> {
    if state.policy.has_feature("external sharing") {
        Ok(())
    } else {
        Err(HttpError::not_found())
    }
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

fn validate_event_identity(name: &str, special_callsign: Option<&str>) -> HttpResult<()> {
    if name.trim().is_empty() || name.trim().chars().count() > 160 {
        return Err(HttpError::bad_request(
            "event name must contain 1 to 160 characters",
        ));
    }
    if let Some(callsign) = special_callsign.filter(|callsign| !callsign.trim().is_empty()) {
        validate_callsign(callsign)?;
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

    #[test]
    fn event_identity_requires_a_bounded_name_and_valid_special_callsign() {
        assert!(validate_event_identity("Field Day", Some("W1AW/7")).is_ok());
        assert!(validate_event_identity("", None).is_err());
        assert!(validate_event_identity(&"x".repeat(161), None).is_err());
        assert!(validate_event_identity("Field Day", Some("not a call sign")).is_err());
    }
}
