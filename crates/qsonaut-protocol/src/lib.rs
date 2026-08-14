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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetupStatus {
    pub setup_required: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubPosition {
    pub id: Uuid,
    pub club_id: Uuid,
    pub name: String,
    pub position_type: String,
    pub seats: i32,
    pub term_years: i32,
    pub election_parity: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubPositionInput {
    pub name: String,
    pub position_type: String,
    pub seats: i32,
    pub term_years: i32,
    pub election_parity: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubPositionAssignment {
    pub id: Uuid,
    pub position_id: Uuid,
    pub user_id: Uuid,
    pub callsign: String,
    pub display_name: String,
    pub seat_number: i32,
    pub starts_on: NaiveDate,
    pub ends_on: NaiveDate,
    pub selection_method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubPositionAssignmentInput {
    pub position_id: Uuid,
    pub user_id: Uuid,
    pub seat_number: i32,
    pub starts_on: NaiveDate,
    pub ends_on: NaiveDate,
    pub selection_method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubElection {
    pub id: Uuid,
    pub club_id: Uuid,
    pub title: String,
    pub election_year: i32,
    pub status: String,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub notes: String,
    pub position_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubElectionInput {
    pub title: String,
    pub election_year: i32,
    #[serde(default = "default_election_status")]
    pub status: String,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub position_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubElectionStatusInput {
    pub status: String,
}

fn default_election_status() -> String {
    "planned".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ClubGovernance {
    pub positions: Vec<ClubPosition>,
    pub assignments: Vec<ClubPositionAssignment>,
    pub elections: Vec<ClubElection>,
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
    pub contest_config: serde_json::Value,
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
    pub event_id: Option<Uuid>,
    pub event_name: Option<String>,
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
        contest_templates: Vec<ContestTemplate>,
        channel_messages: Vec<ChannelMessage>,
    },
    PresenceAccepted(StationPresence),
    LogAccepted(QsoLog),
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

fn default_json_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub error: String,
}
