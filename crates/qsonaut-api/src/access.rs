use crate::{
    AppState,
    auth::{normalize_call, require_admin, validate_callsign},
    error::{HttpError, HttpResult},
};
use axum::{
    Json,
    extract::{Path, State},
};
use axum_extra::extract::cookie::CookieJar;
use base64::Engine;
use chrono::{Duration, Utc};
use qsonaut_protocol::{
    AccessCallsignLookup, AccessChallenge, AccessDecisionResult, AccessRequest,
    AccessRequestDecisionInput, AccessRequestInput,
};
use qsonaut_store::NewAccessRequest;
use rand::{Rng, RngCore};
use serde::Deserialize;
use uuid::Uuid;

struct TechQuestion {
    prompt: &'static str,
    answers: &'static [&'static str],
}

const TECH_QUESTIONS: &[TechQuestion] = &[
    TechQuestion {
        prompt: "What does RF stand for?",
        answers: &["radio frequency"],
    },
    TechQuestion {
        prompt: "What unit is used to measure electrical resistance?",
        answers: &["ohm", "ohms"],
    },
    TechQuestion {
        prompt: "What component commonly transmits and receives radio signals?",
        answers: &["antenna", "an antenna", "aerial"],
    },
    TechQuestion {
        prompt: "What does PTT stand for?",
        answers: &["push to talk", "push-to-talk"],
    },
    TechQuestion {
        prompt: "What is the common abbreviation for decibel?",
        answers: &["db"],
    },
    TechQuestion {
        prompt: "What does SWR stand for?",
        answers: &["standing wave ratio"],
    },
];

#[derive(Debug, Deserialize)]
struct HamDbResponse {
    hamdb: HamDbEnvelope,
}

#[derive(Debug, Deserialize)]
struct HamDbEnvelope {
    callsign: HamDbCallsign,
}

#[derive(Debug, Deserialize)]
struct HamDbCallsign {
    call: String,
    #[serde(default, alias = "fname")]
    first_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    grid: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    status: String,
}

#[utoipa::path(get, path = "/api/v1/access/challenge", tag = "access", responses((status = 200, body = AccessChallenge)))]
pub(crate) async fn challenge(State(state): State<AppState>) -> HttpResult<Json<AccessChallenge>> {
    let index = rand::rng().random_range(0..TECH_QUESTIONS.len());
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(10);
    state
        .store
        .create_access_challenge(
            id,
            i32::try_from(index).map_err(|_| HttpError::internal())?,
            expires_at,
        )
        .await?;
    Ok(Json(AccessChallenge {
        id,
        question: TECH_QUESTIONS[index].prompt.to_owned(),
        expires_at,
        attempts_remaining: 3,
    }))
}

#[utoipa::path(get, path = "/api/v1/access/lookup/{callsign}", tag = "access", params(("callsign" = String, Path)), responses((status = 200, body = AccessCallsignLookup), (status = 400), (status = 502)))]
pub(crate) async fn lookup_callsign(
    Path(callsign): Path<String>,
) -> HttpResult<Json<AccessCallsignLookup>> {
    let callsign = normalize_call(&callsign);
    validate_callsign(&callsign)?;
    let ham = lookup_hamdb(&callsign).await?;
    Ok(Json(AccessCallsignLookup {
        callsign: ham.call,
        display_name: format!("{} {}", ham.first_name, ham.name).trim().to_owned(),
        grid: ham.grid,
        license_class: ham.class,
        license_status: ham.status,
    }))
}

#[utoipa::path(post, path = "/api/v1/access/request", tag = "access", request_body = AccessRequestInput, responses((status = 200, body = AccessRequest), (status = 400), (status = 502)))]
pub(crate) async fn submit(
    State(state): State<AppState>,
    Json(input): Json<AccessRequestInput>,
) -> HttpResult<Json<AccessRequest>> {
    let callsign = normalize_call(&input.callsign);
    validate_callsign(&callsign)?;
    validate_email(&input.email)?;
    validate_optional_field("club name", &input.club_name, 160)?;
    validate_optional_field("referral source", &input.referral_source, 240)?;
    let Some((question_index, attempts_remaining)) = state
        .store
        .take_access_challenge_attempt(input.challenge_id)
        .await?
    else {
        return Err(HttpError::bad_request(
            "the technical challenge has expired; request a new one",
        ));
    };
    if !answer_is_correct(question_index, &input.challenge_answer) {
        if attempts_remaining == 0 {
            state
                .store
                .consume_access_challenge(input.challenge_id)
                .await?;
            return Err(HttpError::bad_request(
                "incorrect answer; request a new challenge",
            ));
        }
        return Err(HttpError::bad_request(format!(
            "incorrect answer; {attempts_remaining} attempts remaining"
        )));
    }
    state
        .store
        .consume_access_challenge(input.challenge_id)
        .await?;
    let ham = lookup_hamdb(&callsign).await?;
    let display_name = format!("{} {}", ham.first_name, ham.name).trim().to_owned();
    let new_request = NewAccessRequest {
        id: Uuid::new_v4(),
        callsign: &callsign,
        email: input.email.trim(),
        club_name: input.club_name.trim(),
        referral_source: input.referral_source.trim(),
        hamdb_display_name: &display_name,
        hamdb_grid: &ham.grid,
        hamdb_license_class: &ham.class,
        hamdb_license_status: &ham.status,
    };
    let request = state
        .store
        .create_access_request(&new_request)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database) = &error
                && database.constraint() == Some("access_requests_pending_callsign_idx")
            {
                return HttpError::conflict("a request for this callsign is already pending");
            }
            HttpError::from(error)
        })?;
    tracing::info!(
        request_id = %request.id,
        callsign = %request.callsign,
        club_requested = !request.club_name.is_empty(),
        "access request submitted"
    );
    Ok(Json(request))
}

#[utoipa::path(get, path = "/api/v1/access/requests", tag = "access", responses((status = 200, body = [AccessRequest]), (status = 403)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<AccessRequest>>> {
    require_admin(&state, &jar).await?;
    Ok(Json(state.store.access_requests().await?))
}

#[utoipa::path(patch, path = "/api/v1/access/requests/{request_id}", tag = "access", params(("request_id" = Uuid, Path)), request_body = AccessRequestDecisionInput, responses((status = 200, body = AccessDecisionResult), (status = 403), (status = 404)))]
pub(crate) async fn decide(
    Path(request_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<AccessRequestDecisionInput>,
) -> HttpResult<Json<AccessDecisionResult>> {
    let reviewer = require_admin(&state, &jar).await?;
    if !["approved", "rejected"].contains(&input.decision.as_str()) {
        return Err(HttpError::bad_request(
            "decision must be approved or rejected",
        ));
    }
    if input.decision == "approved" {
        let temporary_password = temporary_password();
        let password_hash = crate::auth::hash_password(temporary_password.clone()).await?;
        let Some((request, user)) = state
            .store
            .approve_access_request(request_id, reviewer.id, &password_hash)
            .await
            .map_err(|error| {
                if let sqlx::Error::Database(database) = &error
                    && database.is_unique_violation()
                {
                    return HttpError::conflict("an account already exists for this callsign");
                }
                HttpError::from(error)
            })?
        else {
            return Err(HttpError::not_found());
        };
        tracing::info!(
            request_id = %request.id,
            callsign = %request.callsign,
            reviewer_id = %reviewer.id,
            user_id = %user.id,
            "access request approved and account created"
        );
        return Ok(Json(AccessDecisionResult {
            request,
            user: Some(user),
            temporary_password: Some(temporary_password),
        }));
    }
    let request = state
        .store
        .decide_access_request(request_id, &input.decision, reviewer.id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    tracing::info!(
        request_id = %request.id,
        callsign = %request.callsign,
        reviewer_id = %reviewer.id,
        "access request rejected"
    );
    Ok(Json(AccessDecisionResult {
        request,
        user: None,
        temporary_password: None,
    }))
}

fn temporary_password() -> String {
    let mut bytes = [0_u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

async fn lookup_hamdb(callsign: &str) -> HttpResult<HamDbCallsign> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("QSONaut-Server/0.1")
        .build()
        .map_err(|_| HttpError::bad_gateway("could not initialize HamDB client"))?;
    let response = client
        .get(format!("https://api.hamdb.org/{callsign}/json/QSONaut"))
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(%error, "HamDB access lookup failed");
            HttpError::bad_gateway("HamDB could not be reached")
        })?
        .error_for_status()
        .map_err(|error| {
            tracing::warn!(%error, "HamDB rejected access lookup");
            HttpError::bad_gateway("HamDB rejected the lookup")
        })?;
    let response = response.json::<HamDbResponse>().await.map_err(|error| {
        tracing::warn!(%error, "HamDB access response was invalid");
        HttpError::bad_gateway("HamDB returned invalid license data")
    })?;
    if response.hamdb.callsign.call.trim().is_empty()
        || normalize_call(&response.hamdb.callsign.call) != callsign
        || response
            .hamdb
            .callsign
            .status
            .eq_ignore_ascii_case("invalid")
    {
        return Err(HttpError::bad_request(
            "HamDB did not find a valid callsign",
        ));
    }
    Ok(response.hamdb.callsign)
}

fn validate_email(email: &str) -> HttpResult<()> {
    let email = email.trim();
    if !(3..=254).contains(&email.len())
        || email.starts_with('@')
        || email.ends_with('@')
        || !email.contains('@')
    {
        return Err(HttpError::bad_request("enter a valid email address"));
    }
    Ok(())
}

fn validate_optional_field(label: &str, value: &str, max_len: usize) -> HttpResult<()> {
    if value.trim().chars().count() > max_len {
        return Err(HttpError::bad_request(format!(
            "{label} must be {max_len} characters or fewer"
        )));
    }
    Ok(())
}

fn normalized_answer(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn answer_is_correct(question_index: i32, answer: &str) -> bool {
    let Some(question) = usize::try_from(question_index)
        .ok()
        .and_then(|index| TECH_QUESTIONS.get(index))
    else {
        return false;
    };
    let answer = normalized_answer(answer);
    !answer.is_empty()
        && question
            .answers
            .iter()
            .any(|expected| normalized_answer(expected) == answer)
}

#[cfg(test)]
mod tests {
    use super::{TECH_QUESTIONS, answer_is_correct, normalized_answer};

    #[test]
    fn technical_answers_ignore_case_spacing_and_punctuation() {
        assert!(answer_is_correct(0, " Radio-Frequency! "));
        assert!(answer_is_correct(3, "PUSH TO TALK"));
    }

    #[test]
    fn technical_answers_accept_documented_alternatives() {
        assert!(answer_is_correct(1, "ohms"));
        assert!(answer_is_correct(2, "an antenna"));
    }

    #[test]
    fn technical_answers_reject_wrong_empty_and_unknown_questions() {
        assert!(!answer_is_correct(0, "audio frequency"));
        assert!(!answer_is_correct(0, "---"));
        assert!(!answer_is_correct(-1, "radio frequency"));
        assert!(!answer_is_correct(
            i32::try_from(TECH_QUESTIONS.len()).unwrap(),
            "radio frequency"
        ));
    }

    #[test]
    fn answer_normalization_is_ascii_and_predictable() {
        assert_eq!(normalized_answer(" Push-to-talk! "), "pushtotalk");
    }
}
