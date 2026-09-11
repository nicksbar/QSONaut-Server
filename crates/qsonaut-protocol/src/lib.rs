//! Public HTTP and event-stream contracts for `QSONaut` Server clients.
//!
//! Persistence records and server implementation details do not belong here.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const API_VERSION: &str = "v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: ServiceStatus,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ServiceInfo {
    pub name: String,
    pub api_version: String,
    pub purpose: String,
    pub manages: Vec<String>,
    pub excludes: Vec<String>,
}

/// Deployment capabilities advertised to clients and the management UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ServerCapabilities {
    pub edition: String,
    pub max_clubs: Option<i64>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetupStatus {
    pub setup_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessChallenge {
    pub id: Uuid,
    pub question: String,
    pub expires_at: DateTime<Utc>,
    pub attempts_remaining: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessCallsignLookup {
    pub callsign: String,
    pub display_name: String,
    pub grid: String,
    pub license_class: String,
    pub license_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessRequestInput {
    pub challenge_id: Uuid,
    pub challenge_answer: String,
    pub callsign: String,
    pub email: String,
    #[serde(default)]
    pub club_name: String,
    #[serde(default)]
    pub referral_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessRequest {
    pub id: Uuid,
    pub callsign: String,
    pub email: String,
    pub club_name: String,
    pub referral_source: String,
    pub hamdb_display_name: String,
    pub hamdb_grid: String,
    pub hamdb_license_class: String,
    pub hamdb_license_status: String,
    pub status: String,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessRequestDecisionInput {
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessDecisionResult {
    pub request: AccessRequest,
    pub user: Option<CurrentUser>,
    pub temporary_password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Credentials {
    pub callsign: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BootstrapRequest {
    pub callsign: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CurrentUser {
    pub id: Uuid,
    pub callsign: String,
    pub display_name: String,
    pub global_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProfileUpdateInput {
    pub display_name: String,
    #[serde(default)]
    pub grid: String,
    #[serde(default)]
    pub qth: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub middle_name: String,
    #[serde(default)]
    pub surname: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub license_class: String,
    #[serde(default)]
    pub license_status: String,
    #[serde(default)]
    pub license_expires_on: Option<NaiveDate>,
    #[serde(default)]
    pub address_line_1: String,
    #[serde(default)]
    pub address_line_2: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub postal_code: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub latitude: String,
    #[serde(default)]
    pub longitude: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub user: CurrentUser,
    pub grid: String,
    pub qth: String,
    pub first_name: String,
    pub middle_name: String,
    pub surname: String,
    pub suffix: String,
    pub license_class: String,
    pub license_status: String,
    pub license_expires_on: Option<NaiveDate>,
    pub address_line_1: String,
    pub address_line_2: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub latitude: String,
    pub longitude: String,
    pub hamdb_fetched_at: Option<DateTime<Utc>>,
    pub hamdb_last_error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ActivitySummary {
    pub scope: String,
    pub scope_id: Option<Uuid>,
    pub period_days: i32,
    pub qso_count: i64,
    pub unique_callsigns: i64,
    pub band_count: i64,
    pub mode_count: i64,
    pub points: i64,
    pub last_qso_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ActivityVisibility {
    pub id: Uuid,
    pub user_id: Uuid,
    pub scope: String,
    pub scope_id: Option<Uuid>,
    pub visibility: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ActivityVisibilityInput {
    pub scope: String,
    pub scope_id: Option<Uuid>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeviceCredentials {
    pub callsign: String,
    pub password: String,
    pub device_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeviceRegistration {
    pub device_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeviceToken {
    pub token: String,
    pub user: CurrentUser,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeviceTokenRecord {
    pub id: Uuid,
    pub device_name: String,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberInput {
    pub callsign: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberUpdateInput {
    pub display_name: String,
    #[serde(default)]
    pub global_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PasswordResetInput {
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberClubRole {
    pub club_id: Uuid,
    pub club_name: String,
    pub club_callsign: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MemberDetail {
    pub user: CurrentUser,
    pub memberships: Vec<MemberClubRole>,
    pub stations: Vec<StationPresence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubMembership {
    pub club_id: Uuid,
    pub user_id: Uuid,
    pub callsign: String,
    pub display_name: String,
    pub role: String,
    pub membership_status: String,
    pub dues_status: String,
    pub membership_number: Option<String>,
    pub renewal_due_on: Option<NaiveDate>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubMembershipInput {
    pub user_id: Uuid,
    pub role: String,
    #[serde(default)]
    pub membership_status: Option<String>,
    #[serde(default)]
    pub dues_status: Option<String>,
    #[serde(default)]
    pub membership_number: Option<String>,
    #[serde(default)]
    pub renewal_due_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubJoinRequest {
    pub id: Uuid,
    pub club_id: Uuid,
    pub user_id: Uuid,
    pub callsign: String,
    pub display_name: String,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubJoinDecisionInput {
    pub decision: String,
    #[serde(default = "default_operator_role")]
    pub role: String,
}

fn default_operator_role() -> String {
    "operator".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: String,
    pub member_count: i64,
    pub renewal_attention_count: i64,
    pub my_role: Option<String>,
    pub my_membership_status: Option<String>,
    pub join_request_status: Option<String>,
    pub can_manage: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubInput {
    pub name: String,
    pub callsign: Option<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ManagedCallsign {
    pub id: Uuid,
    pub callsign: String,
    pub identity_type: String,
    pub owner_user_id: Option<Uuid>,
    pub club_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub status: String,
    pub effective_from: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub verification_status: String,
    pub authority: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ManagedCallsignInput {
    pub event_id: Uuid,
    pub callsign: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub authority_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ManagedCallsignStatusInput {
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventParticipant {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub club_id: Uuid,
    pub callsign_id: Uuid,
    pub operator_callsign: String,
    pub operating_callsign: String,
    pub role: String,
    pub status: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub station_label: String,
    pub band: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventParticipantInput {
    pub user_id: Uuid,
    pub callsign_id: Uuid,
    pub operator_callsign: String,
    pub role: String,
    #[serde(default = "default_participant_status")]
    pub status: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub station_label: String,
    pub band: Option<String>,
    pub mode: Option<String>,
}

fn default_participant_status() -> String {
    "active".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Draft,
    Scheduled,
    Active,
    Completed,
    Cancelled,
}

impl EventStatus {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Scheduled => "scheduled",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Event {
    pub id: Uuid,
    pub club_id: Uuid,
    pub name: String,
    pub contest_name: String,
    pub special_callsign: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: EventStatus,
    pub contest_template_id: Option<Uuid>,
    pub contest_definition_version: Option<i32>,
    pub contest_config: serde_json::Value,
    pub participant_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventScore {
    pub event_id: Uuid,
    pub total_points: i64,
    pub qso_count: i64,
    pub duplicate_count: i64,
    pub multiplier_values: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EventInput {
    pub club_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub contest_name: String,
    pub special_callsign: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: EventStatus,
    #[serde(default)]
    pub contest_template_id: Option<Uuid>,
    #[serde(default = "default_json_object")]
    pub contest_config: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EventStatusInput {
    pub status: EventStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventUpdateInput {
    pub club_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub contest_name: String,
    pub special_callsign: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: EventStatus,
    #[serde(default)]
    pub contest_template_id: Option<Uuid>,
    #[serde(default = "default_json_object")]
    pub contest_config: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ContestTemplate {
    pub id: Uuid,
    pub contest_type: String,
    pub name: String,
    pub description: String,
    pub organization: String,
    pub icon: String,
    pub rules_url: String,
    pub definition_version: i32,
    pub is_builtin: bool,
    pub scoring_rules: serde_json::Value,
    pub required_fields: serde_json::Value,
    pub validation_rules: serde_json::Value,
    pub schedule: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StationPresence {
    pub id: Uuid,
    pub user_id: Uuid,
    pub callsign: String,
    pub display_name: String,
    pub instance_id: Uuid,
    pub station_label: String,
    pub radio_manufacturer: Option<String>,
    pub radio_model: Option<String>,
    pub frequency_hz: Option<i64>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub qsonaut_version: String,
    pub platform: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StationPresenceInput {
    pub instance_id: Uuid,
    #[serde(default)]
    pub station_label: String,
    pub radio_manufacturer: Option<String>,
    pub radio_model: Option<String>,
    pub frequency_hz: Option<i64>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub qsonaut_version: String,
    pub platform: String,
    pub status: String,
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QsoLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operator_callsign: String,
    pub operating_callsign: Option<String>,
    pub callsign_id: Option<Uuid>,
    pub contest_template_id: Option<Uuid>,
    pub contest_definition_version: Option<i32>,
    pub contest_config: serde_json::Value,
    pub is_duplicate: bool,
    pub multipliers: serde_json::Value,
    pub scoring_version: Option<String>,
    pub scoring_explanation: String,
    pub event_id: Option<Uuid>,
    pub event_name: Option<String>,
    pub visibility: String,
    pub visibility_club_id: Option<Uuid>,
    pub idempotency_key: Uuid,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub frequency_hz: Option<i64>,
    pub occurred_at: DateTime<Utc>,
    pub rst_sent: Option<String>,
    pub rst_received: Option<String>,
    pub exchange: serde_json::Value,
    pub points: i32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QsoLogInput {
    pub event_id: Option<Uuid>,
    #[serde(default)]
    pub operating_callsign: Option<String>,
    #[serde(default)]
    pub callsign_id: Option<Uuid>,
    #[serde(default = "default_qso_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub visibility_club_id: Option<Uuid>,
    pub idempotency_key: Uuid,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub frequency_hz: Option<i64>,
    pub occurred_at: DateTime<Utc>,
    pub rst_sent: Option<String>,
    pub rst_received: Option<String>,
    #[serde(default = "default_json_object")]
    pub exchange: serde_json::Value,
    #[serde(default)]
    pub points: i32,
    #[serde(default = "default_qso_source")]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ShareLinkInput {
    #[serde(default = "default_share_expiry_days")]
    pub expires_in_days: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ShareLink {
    pub id: Uuid,
    pub share_path: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SharedQsoDetail {
    pub operator_callsign: String,
    pub event_name: Option<String>,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub frequency_hz: Option<i64>,
    pub occurred_at: DateTime<Utc>,
    pub rst_sent: Option<String>,
    pub rst_received: Option<String>,
    pub exchange: serde_json::Value,
    pub points: i32,
    pub source: String,
}

impl From<QsoLog> for SharedQsoDetail {
    fn from(log: QsoLog) -> Self {
        Self {
            operator_callsign: log.operator_callsign,
            event_name: log.event_name,
            callsign: log.callsign,
            band: log.band,
            mode: log.mode,
            frequency_hz: log.frequency_hz,
            occurred_at: log.occurred_at,
            rst_sent: log.rst_sent,
            rst_received: log.rst_received,
            exchange: log.exchange,
            points: log.points,
            source: log.source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ShareLinkRecord {
    pub id: Uuid,
    pub qso_log_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub worked_callsign: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

fn default_share_expiry_days() -> i64 {
    7
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ChannelMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub author_callsign: String,
    pub event_id: Option<Uuid>,
    pub channel: String,
    pub message: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ChannelMessageInput {
    pub event_id: Option<Uuid>,
    pub channel: String,
    pub message: String,
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DiagnosticReportInput {
    pub instance_id: Uuid,
    pub category: String,
    pub summary: String,
    #[serde(default = "default_json_object")]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DiagnosticReport {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operator_callsign: String,
    pub instance_id: Uuid,
    pub category: String,
    pub summary: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvelope {
    pub protocol_version: String,
    pub event_id: Uuid,
    #[serde(flatten)]
    pub message: ClientMessage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello { client_version: String },
    Sync,
    Presence(StationPresenceInput),
    Log(QsoLogInput),
    Diagnostic(DiagnosticReportInput),
    ChannelMessage(ChannelMessageInput),
    Ping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerEnvelope {
    pub protocol_version: String,
    pub event_id: Uuid,
    #[serde(flatten)]
    pub message: ServerMessage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        user: CurrentUser,
    },
    Snapshot {
        events: Vec<Event>,
        clubs: Vec<Club>,
        contest_templates: Vec<ContestTemplate>,
        #[serde(default)]
        event_scores: Vec<EventScore>,
        identities: Vec<ManagedCallsign>,
        participants: Vec<EventParticipant>,
        channel_messages: Vec<ChannelMessage>,
    },
    PresenceAccepted(StationPresence),
    LogAccepted {
        qso: Box<QsoLog>,
        #[serde(default)]
        event_score: Option<EventScore>,
    },
    DiagnosticAccepted(DiagnosticReport),
    ChannelMessageAccepted(ChannelMessage),
    ChannelMessagePublished(ChannelMessage),
    Ack,
    Pong,
    Error {
        message: String,
    },
}

fn default_qso_source() -> String {
    "qsonaut".to_owned()
}

fn default_qso_visibility() -> String {
    "private".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{QsoLog, SharedQsoDetail};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn public_qso_detail_omits_internal_identifiers_and_visibility_state() {
        let detail = SharedQsoDetail::from(QsoLog {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            operator_callsign: "N7UF".to_owned(),
            operating_callsign: None,
            callsign_id: None,
            contest_template_id: None,
            contest_definition_version: None,
            contest_config: serde_json::json!({}),
            is_duplicate: false,
            multipliers: serde_json::json!({}),
            scoring_version: None,
            scoring_explanation: String::new(),
            event_id: Some(Uuid::new_v4()),
            event_name: Some("Field Day".to_owned()),
            visibility: "private".to_owned(),
            visibility_club_id: Some(Uuid::new_v4()),
            idempotency_key: Uuid::new_v4(),
            callsign: "W1AW".to_owned(),
            band: "20m".to_owned(),
            mode: "FT8".to_owned(),
            frequency_hz: Some(14_074_000),
            occurred_at: Utc::now(),
            rst_sent: Some("-10".to_owned()),
            rst_received: Some("-12".to_owned()),
            exchange: serde_json::json!({}),
            points: 1,
            source: "qsonaut".to_owned(),
        });
        let value = serde_json::to_value(detail).expect("shared detail serializes");
        for internal in [
            "id",
            "user_id",
            "event_id",
            "visibility",
            "visibility_club_id",
            "idempotency_key",
        ] {
            assert!(value.get(internal).is_none(), "leaked {internal}");
        }
    }
}

fn default_json_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub error: String,
}
