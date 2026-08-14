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
    ChannelMessage, Club, ClubInput, ClubMembership, ClubMembershipInput, ContestTemplate,
    CurrentUser, Event, EventInput, EventStatusInput, MemberDetail, MemberInput, MemberUpdateInput,
    PasswordResetInput, QsoLog, QsoLogInput, StationPresence, StationPresenceInput,
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
    require_user(&state, &jar).await?;
    Ok(Json(state.store.club_members(club_id).await?))
}
#[utoipa::path(put, path = "/api/v1/clubs/{club_id}/members", tag = "management", params(("club_id" = Uuid, Path)), request_body = ClubMembershipInput, responses((status = 200, body = ClubMembership)))]
pub(crate) async fn set_club_member(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ClubMembershipInput>,
) -> HttpResult<Json<ClubMembership>> {
    require_admin(&state, &jar).await?;
    if !["owner", "coordinator", "operator", "observer"].contains(&input.role.as_str()) {
        return Err(HttpError::bad_request("invalid club role"));
    }
    Ok(Json(
        state
            .store
            .set_club_member(club_id, input.user_id, &input.role)
            .await?,
    ))
}
#[utoipa::path(delete, path = "/api/v1/clubs/{club_id}/members/{member_id}", tag = "management", params(("club_id" = Uuid, Path), ("member_id" = Uuid, Path)), responses((status = 204, description = "Membership removed"), (status = 404, description = "Membership not found")))]
pub(crate) async fn remove_club_member(
    Path((club_id, member_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<StatusCode> {
    require_admin(&state, &jar).await?;
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
    require_user(&state, &jar).await?;
    Ok(Json(state.store.clubs().await?))
}
#[utoipa::path(post, path = "/api/v1/clubs", tag = "management", request_body = ClubInput, responses((status = 200, body = Club)))]
pub(crate) async fn create_club(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(mut input): Json<ClubInput>,
) -> HttpResult<Json<Club>> {
    let user = require_admin(&state, &jar).await?;
    if input.name.trim().is_empty() {
        return Err(HttpError::bad_request("club name is required"));
    }
    input.callsign = input
        .callsign
        .map(|call| call.trim().to_ascii_uppercase())
        .filter(|call| !call.is_empty());
    Ok(Json(state.store.create_club(&input, user.id).await?))
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

#[cfg(test)]
mod tests {
    use super::*;

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
