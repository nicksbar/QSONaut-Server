//! `PostgreSQL` persistence for `QSONaut` Server.
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, NaiveDate, Utc};
use qsonaut_protocol::{
    AccessRequest, ActivityMapPoint, ActivityVisibility, ActivityVisibilityInput, ChannelMessage,
    ChannelMessageInput, Club, ClubInput, ClubJoinRequest, ClubMembership, ClubMembershipInput,
    ContestTemplate, CurrentUser, DiagnosticReport, DiagnosticReportInput, Event, EventInput,
    EventParticipant, EventParticipantInput, EventScore, EventStatus, EventUpdateInput,
    ManagedCallsign, ManagedCallsignInput, MemberClubRole, ProfileUpdateInput, QsoLog, QsoLogInput,
    ShareLinkRecord, StationPresence, StationPresenceInput, UserProfile,
};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::collections::HashMap;
use uuid::Uuid;

fn should_preserve_last_administrator(
    administrator_count: i64,
    target_is_administrator: bool,
) -> bool {
    target_is_administrator && administrator_count <= 1
}

fn maidenhead_center(grid: &str) -> Option<(String, f64, f64)> {
    let normalized = grid.trim().to_ascii_uppercase();
    let bytes = normalized.as_bytes();
    if !(bytes.len() == 4 || bytes.len() == 6)
        || !(b'A'..=b'R').contains(&bytes[0])
        || !(b'A'..=b'R').contains(&bytes[1])
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
        || (bytes.len() == 6 && (!(b'A'..=b'X').contains(&bytes[4]) || !(b'A'..=b'X').contains(&bytes[5])))
    {
        return None;
    }
    let mut longitude = -180.0 + f64::from(bytes[0] - b'A') * 20.0 + f64::from(bytes[2] - b'0') * 2.0;
    let mut latitude = -90.0 + f64::from(bytes[1] - b'A') * 10.0 + f64::from(bytes[3] - b'0');
    let (width, height) = if bytes.len() == 6 {
        longitude += f64::from(bytes[4] - b'A') * (5.0 / 60.0);
        latitude += f64::from(bytes[5] - b'A') * (2.5 / 60.0);
        (5.0 / 60.0, 2.5 / 60.0)
    } else {
        (2.0, 1.0)
    };
    Some((normalized, latitude + height / 2.0, longitude + width / 2.0))
}

fn grid_from_exchange(exchange: &Value) -> Option<&str> {
    const GRID_KEYS: [&str; 3] = ["grid", "grid_square", "gridsquare"];
    GRID_KEYS
        .into_iter()
        .find_map(|key| exchange.get(key).and_then(Value::as_str))
        .or_else(|| {
            ["fields_received", "fields_sent"]
                .into_iter()
                .find_map(|section| {
                    exchange
                        .get(section)
                        .and_then(|fields| GRID_KEYS.into_iter().find_map(|key| fields.get(key).and_then(Value::as_str)))
                })
        })
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    club_id: Uuid,
    name: String,
    contest_name: String,
    special_callsign: Option<String>,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    status: String,
    contest_template_id: Option<Uuid>,
    contest_definition_version: Option<i32>,
    contest_config: Value,
    participant_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationApprovalRecord {
    pub device_name: String,
    pub client_id: String,
    pub client_version: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeviceAuthorizationGrantInput<'a> {
    pub id: Uuid,
    pub client_id: &'a str,
    pub device_name: &'a str,
    pub client_version: &'a str,
    pub device_code_hash: &'a [u8],
    pub user_code_hash: &'a [u8],
    pub expires_at: DateTime<Utc>,
    pub interval_seconds: i32,
}

impl From<EventRow> for Event {
    fn from(row: EventRow) -> Self {
        let status = match row.status.as_str() {
            "scheduled" => EventStatus::Scheduled,
            "active" => EventStatus::Active,
            "completed" => EventStatus::Completed,
            "cancelled" => EventStatus::Cancelled,
            _ => EventStatus::Draft,
        };
        Self {
            id: row.id,
            club_id: row.club_id,
            name: row.name,
            contest_name: row.contest_name,
            special_callsign: row.special_callsign,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            status,
            contest_template_id: row.contest_template_id,
            contest_definition_version: row.contest_definition_version,
            contest_config: row.contest_config,
            participant_count: row.participant_count,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StationPresenceRow {
    id: Uuid,
    user_id: Uuid,
    callsign: String,
    display_name: String,
    instance_id: Uuid,
    station_label: String,
    radio_manufacturer: Option<String>,
    radio_model: Option<String>,
    frequency_hz: Option<i64>,
    band: Option<String>,
    mode: Option<String>,
    qsonaut_version: String,
    platform: String,
    status: String,
    metadata: Value,
    last_seen: DateTime<Utc>,
}

impl From<StationPresenceRow> for StationPresence {
    fn from(row: StationPresenceRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            callsign: row.callsign,
            display_name: row.display_name,
            instance_id: row.instance_id,
            station_label: row.station_label,
            radio_manufacturer: row.radio_manufacturer,
            radio_model: row.radio_model,
            frequency_hz: row.frequency_hz,
            band: row.band,
            mode: row.mode,
            qsonaut_version: row.qsonaut_version,
            platform: row.platform,
            status: row.status,
            metadata: row.metadata,
            last_seen: row.last_seen,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QsoLogRow {
    id: Uuid,
    user_id: Uuid,
    operator_callsign: String,
    operating_callsign: Option<String>,
    callsign_id: Option<Uuid>,
    contest_template_id: Option<Uuid>,
    contest_definition_version: Option<i32>,
    contest_config: Value,
    is_duplicate: bool,
    multipliers: Value,
    scoring_version: Option<String>,
    scoring_explanation: String,
    event_id: Option<Uuid>,
    event_name: Option<String>,
    idempotency_key: Uuid,
    callsign: String,
    band: String,
    mode: String,
    frequency_hz: Option<i64>,
    occurred_at: DateTime<Utc>,
    rst_sent: Option<String>,
    rst_received: Option<String>,
    exchange: Value,
    points: i32,
    source: String,
}

const QSO_LOG_COLUMNS: &str = "q.id, q.user_id, COALESCE(q.operator_callsign,u.callsign) AS operator_callsign, q.operating_callsign, q.callsign_id, q.contest_template_id, q.contest_definition_version, q.contest_config, q.is_duplicate, q.multipliers, q.scoring_version, q.scoring_explanation, q.event_id, e.name AS event_name, q.idempotency_key, q.callsign, q.band, q.mode, q.frequency_hz, q.occurred_at, q.rst_sent, q.rst_received, q.exchange, q.points, q.source";

fn qso_log_query(from_and_where: &str) -> String {
    format!("SELECT {QSO_LOG_COLUMNS} {from_and_where}")
}

#[derive(sqlx::FromRow)]
struct UserProfileRow {
    id: Uuid,
    callsign: String,
    display_name: String,
    global_role: String,
    grid: String,
    qth: String,
    first_name: String,
    middle_name: String,
    surname: String,
    suffix: String,
    license_class: String,
    license_status: String,
    license_expires_on: Option<NaiveDate>,
    address_line_1: String,
    address_line_2: String,
    state: String,
    postal_code: String,
    country: String,
    latitude: String,
    longitude: String,
    hamdb_fetched_at: Option<DateTime<Utc>>,
    hamdb_last_error: String,
}

#[derive(sqlx::FromRow)]
struct AccessRequestRow {
    id: Uuid,
    callsign: String,
    email: String,
    club_name: String,
    referral_source: String,
    hamdb_display_name: String,
    hamdb_grid: String,
    hamdb_license_class: String,
    hamdb_license_status: String,
    status: String,
    reviewed_at: Option<DateTime<Utc>>,
    reviewed_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

pub struct NewAccessRequest<'a> {
    pub id: Uuid,
    pub callsign: &'a str,
    pub email: &'a str,
    pub club_name: &'a str,
    pub referral_source: &'a str,
    pub hamdb_display_name: &'a str,
    pub hamdb_grid: &'a str,
    pub hamdb_license_class: &'a str,
    pub hamdb_license_status: &'a str,
}

impl From<AccessRequestRow> for AccessRequest {
    fn from(row: AccessRequestRow) -> Self {
        Self {
            id: row.id,
            callsign: row.callsign,
            email: row.email,
            club_name: row.club_name,
            referral_source: row.referral_source,
            hamdb_display_name: row.hamdb_display_name,
            hamdb_grid: row.hamdb_grid,
            hamdb_license_class: row.hamdb_license_class,
            hamdb_license_status: row.hamdb_license_status,
            status: row.status,
            reviewed_at: row.reviewed_at,
            reviewed_by: row.reviewed_by,
            created_at: row.created_at,
        }
    }
}

impl From<UserProfileRow> for UserProfile {
    fn from(row: UserProfileRow) -> Self {
        Self {
            user: CurrentUser {
                id: row.id,
                callsign: row.callsign,
                display_name: row.display_name,
                global_role: row.global_role,
            },
            grid: row.grid,
            qth: row.qth,
            first_name: row.first_name,
            middle_name: row.middle_name,
            surname: row.surname,
            suffix: row.suffix,
            license_class: row.license_class,
            license_status: row.license_status,
            license_expires_on: row.license_expires_on,
            address_line_1: row.address_line_1,
            address_line_2: row.address_line_2,
            state: row.state,
            postal_code: row.postal_code,
            country: row.country,
            latitude: row.latitude,
            longitude: row.longitude,
            hamdb_fetched_at: row.hamdb_fetched_at,
            hamdb_last_error: row.hamdb_last_error,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChannelMessageRow {
    id: Uuid,
    user_id: Uuid,
    author_callsign: String,
    event_id: Option<Uuid>,
    channel: String,
    message: String,
    metadata: Value,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ClubJoinRequestRow {
    id: Uuid,
    club_id: Uuid,
    user_id: Uuid,
    callsign: String,
    display_name: String,
    status: String,
    requested_at: DateTime<Utc>,
    reviewed_at: Option<DateTime<Utc>>,
    reviewed_by: Option<Uuid>,
}

impl From<ClubJoinRequestRow> for ClubJoinRequest {
    fn from(row: ClubJoinRequestRow) -> Self {
        Self {
            id: row.id,
            club_id: row.club_id,
            user_id: row.user_id,
            callsign: row.callsign,
            display_name: row.display_name,
            status: row.status,
            requested_at: row.requested_at,
            reviewed_at: row.reviewed_at,
            reviewed_by: row.reviewed_by,
        }
    }
}

impl From<ChannelMessageRow> for ChannelMessage {
    fn from(row: ChannelMessageRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            author_callsign: row.author_callsign,
            event_id: row.event_id,
            channel: row.channel,
            message: row.message,
            metadata: row.metadata,
            created_at: row.created_at,
        }
    }
}

impl From<QsoLogRow> for QsoLog {
    fn from(row: QsoLogRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            operator_callsign: row.operator_callsign,
            operating_callsign: row.operating_callsign,
            callsign_id: row.callsign_id,
            contest_template_id: row.contest_template_id,
            contest_definition_version: row.contest_definition_version,
            contest_config: row.contest_config,
            is_duplicate: row.is_duplicate,
            multipliers: row.multipliers,
            scoring_version: row.scoring_version,
            scoring_explanation: row.scoring_explanation,
            event_id: row.event_id,
            event_name: row.event_name,
            idempotency_key: row.idempotency_key,
            callsign: row.callsign,
            band: row.band,
            mode: row.mode,
            frequency_hz: row.frequency_hz,
            occurred_at: row.occurred_at,
            rst_sent: row.rst_sent,
            rst_received: row.rst_received,
            exchange: row.exchange,
            points: row.points,
            source: row.source,
        }
    }
}

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        // Keep the embedded migration set sensitive to newly added migration files.
        sqlx::migrate!("../../migrations").run(&pool).await?;
        let store = Self { pool };
        store.sync_builtin_contests().await?;
        Ok(store)
    }

    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn setup_required(&self) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count == 0)
    }

    pub async fn create_access_challenge(
        &self,
        id: Uuid,
        question_index: i32,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "WITH expired AS (DELETE FROM access_challenges WHERE expires_at <= now()) INSERT INTO access_challenges (id,question_index,expires_at) VALUES ($1,$2,$3)",
        )
        .bind(id)
        .bind(question_index)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub async fn take_access_challenge_attempt(
        &self,
        id: Uuid,
    ) -> Result<Option<(i32, i16)>, sqlx::Error> {
        sqlx::query_as::<_, (i32, i16)>(
            "UPDATE access_challenges SET attempts_remaining=attempts_remaining-1 WHERE id=$1 AND expires_at > now() AND attempts_remaining > 0 RETURNING question_index,attempts_remaining",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn consume_access_challenge(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM access_challenges WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn create_access_request(
        &self,
        input: &NewAccessRequest<'_>,
    ) -> Result<AccessRequest, sqlx::Error> {
        sqlx::query_as::<_, AccessRequestRow>("INSERT INTO access_requests (id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status,status,reviewed_at,reviewed_by,created_at")
            .bind(input.id).bind(input.callsign).bind(input.email).bind(input.club_name).bind(input.referral_source)
            .bind(input.hamdb_display_name).bind(input.hamdb_grid).bind(input.hamdb_license_class).bind(input.hamdb_license_status)
            .fetch_one(&self.pool).await.map(AccessRequest::from)
    }

    pub async fn access_requests(&self) -> Result<Vec<AccessRequest>, sqlx::Error> {
        sqlx::query_as::<_, AccessRequestRow>("SELECT id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status,status,reviewed_at,reviewed_by,created_at FROM access_requests ORDER BY CASE WHEN status='pending' THEN 0 ELSE 1 END, created_at DESC")
            .fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(AccessRequest::from).collect())
    }

    pub async fn decide_access_request(
        &self,
        id: Uuid,
        decision: &str,
        reviewer_id: Uuid,
    ) -> Result<Option<AccessRequest>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let request = sqlx::query_as::<_, AccessRequestRow>("UPDATE access_requests SET status=$2,reviewed_at=now(),reviewed_by=$3 WHERE id=$1 AND status='pending' RETURNING id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status,status,reviewed_at,reviewed_by,created_at")
            .bind(id).bind(decision).bind(reviewer_id).fetch_optional(&mut *tx).await?;
        if let Some(request) = &request {
            sqlx::query("INSERT INTO audit_events (id,actor_user_id,action,target_type,target_id,metadata) VALUES ($1,$2,$3,'access_request',$4,$5)")
                .bind(Uuid::new_v4()).bind(reviewer_id).bind(format!("access_request_{decision}"))
                .bind(id).bind(serde_json::json!({"callsign": request.callsign})).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(request.map(AccessRequest::from))
    }

    pub async fn approve_access_request(
        &self,
        request_id: Uuid,
        reviewer_id: Uuid,
        password_hash: &str,
    ) -> Result<Option<(AccessRequest, CurrentUser)>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(716736630)")
            .execute(&mut *tx)
            .await?;
        let request = sqlx::query_as::<_, AccessRequestRow>("SELECT id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status,status,reviewed_at,reviewed_by,created_at FROM access_requests WHERE id=$1 AND status='pending' FOR UPDATE")
            .bind(request_id).fetch_optional(&mut *tx).await?;
        let Some(request) = request else {
            tx.rollback().await?;
            return Ok(None);
        };
        let display_name = if request.hamdb_display_name.trim().is_empty() {
            request.callsign.clone()
        } else {
            request.hamdb_display_name.trim().to_owned()
        };
        let user = sqlx::query_as::<_, (Uuid, String, String, String)>("INSERT INTO users (id,callsign,display_name,password_hash,global_role,grid,license_class,license_status,hamdb_fetched_at) VALUES ($1,$2,$3,$4,'member',$5,$6,$7,now()) RETURNING id,callsign,display_name,global_role")
            .bind(Uuid::new_v4()).bind(&request.callsign).bind(display_name).bind(password_hash)
            .bind(&request.hamdb_grid).bind(&request.hamdb_license_class).bind(&request.hamdb_license_status)
            .fetch_one(&mut *tx).await?;
        let reviewed = sqlx::query_as::<_, AccessRequestRow>("UPDATE access_requests SET status='approved',reviewed_at=now(),reviewed_by=$2 WHERE id=$1 RETURNING id,callsign,email,club_name,referral_source,hamdb_display_name,hamdb_grid,hamdb_license_class,hamdb_license_status,status,reviewed_at,reviewed_by,created_at")
            .bind(request_id).bind(reviewer_id).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO audit_events (id,actor_user_id,action,target_type,target_id,metadata) VALUES ($1,$2,'access_request_approved','access_request',$3,$4)")
            .bind(Uuid::new_v4()).bind(reviewer_id).bind(request_id)
            .bind(serde_json::json!({"callsign": reviewed.callsign, "user_id": user.0})).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some((
            AccessRequest::from(reviewed),
            CurrentUser {
                id: user.0,
                callsign: user.1,
                display_name: user.2,
                global_role: user.3,
            },
        )))
    }

    pub async fn create_administrator(
        &self,
        callsign: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<Option<CurrentUser>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        // Serialize first-run setup across all server instances sharing this database.
        sqlx::query("SELECT pg_advisory_xact_lock(716736628)")
            .execute(&mut *tx)
            .await?;
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "INSERT INTO users (id, callsign, display_name, password_hash, global_role) SELECT $1, $2, $3, $4, 'administrator' WHERE NOT EXISTS (SELECT 1 FROM users) RETURNING id, callsign, display_name, global_role"
        ).bind(id).bind(callsign).bind(display_name).bind(password_hash)
        .fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        Ok(row.map(|row| CurrentUser {
            id: row.0,
            callsign: row.1,
            display_name: row.2,
            global_role: row.3,
        }))
    }

    pub async fn user_credentials(
        &self,
        callsign: &str,
    ) -> Result<Option<(CurrentUser, String)>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
            "SELECT id, callsign, display_name, global_role, password_hash FROM users WHERE callsign = $1"
        ).bind(callsign).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| {
            (
                CurrentUser {
                    id: r.0,
                    callsign: r.1,
                    display_name: r.2,
                    global_role: r.3,
                },
                r.4,
            )
        }))
    }

    pub async fn users(&self) -> Result<Vec<CurrentUser>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT id, callsign, display_name, global_role FROM users ORDER BY callsign",
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| CurrentUser {
                    id: r.0,
                    callsign: r.1,
                    display_name: r.2,
                    global_role: r.3,
                })
                .collect()
        })
    }

    pub async fn user(&self, user_id: Uuid) -> Result<Option<CurrentUser>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT id, callsign, display_name, global_role FROM users WHERE id=$1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| {
            row.map(|r| CurrentUser {
                id: r.0,
                callsign: r.1,
                display_name: r.2,
                global_role: r.3,
            })
        })
    }

    pub async fn user_profile(&self, user_id: Uuid) -> Result<Option<UserProfile>, sqlx::Error> {
        sqlx::query_as::<_, UserProfileRow>("SELECT id,callsign,display_name,global_role,grid,qth,first_name,middle_name,surname,suffix,license_class,license_status,license_expires_on,address_line_1,address_line_2,state,postal_code,country,latitude,longitude,hamdb_fetched_at,hamdb_last_error FROM users WHERE id=$1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(UserProfile::from))
    }

    pub async fn update_user_profile(
        &self,
        user_id: Uuid,
        input: &ProfileUpdateInput,
    ) -> Result<Option<UserProfile>, sqlx::Error> {
        sqlx::query("UPDATE users SET display_name=$2,grid=$3,qth=$4,first_name=$5,middle_name=$6,surname=$7,suffix=$8,license_class=$9,license_status=$10,license_expires_on=$11,address_line_1=$12,address_line_2=$13,state=$14,postal_code=$15,country=$16,latitude=$17,longitude=$18 WHERE id=$1")
            .bind(user_id)
            .bind(input.display_name.trim())
            .bind(input.grid.trim().to_ascii_uppercase())
            .bind(input.qth.trim())
            .bind(input.first_name.trim())
            .bind(input.middle_name.trim())
            .bind(input.surname.trim())
            .bind(input.suffix.trim())
            .bind(input.license_class.trim())
            .bind(input.license_status.trim())
            .bind(input.license_expires_on)
            .bind(input.address_line_1.trim())
            .bind(input.address_line_2.trim())
            .bind(input.state.trim())
            .bind(input.postal_code.trim())
            .bind(input.country.trim())
            .bind(input.latitude.trim())
            .bind(input.longitude.trim())
            .execute(&self.pool)
            .await?;
        self.user_profile(user_id).await
    }

    pub async fn update_hamdb_profile(
        &self,
        user_id: Uuid,
        profile: &ProfileUpdateInput,
    ) -> Result<Option<UserProfile>, sqlx::Error> {
        sqlx::query("UPDATE users SET grid=$2,first_name=$3,middle_name=$4,surname=$5,suffix=$6,license_class=$7,license_status=$8,license_expires_on=$9,address_line_1=$10,address_line_2=$11,state=$12,postal_code=$13,country=$14,latitude=$15,longitude=$16,hamdb_fetched_at=now(),hamdb_last_error='' WHERE id=$1")
            .bind(user_id)
            .bind(profile.grid.trim().to_ascii_uppercase())
            .bind(profile.first_name.trim())
            .bind(profile.middle_name.trim())
            .bind(profile.surname.trim())
            .bind(profile.suffix.trim())
            .bind(profile.license_class.trim())
            .bind(profile.license_status.trim())
            .bind(profile.license_expires_on)
            .bind(profile.address_line_1.trim())
            .bind(profile.address_line_2.trim())
            .bind(profile.state.trim())
            .bind(profile.postal_code.trim())
            .bind(profile.country.trim())
            .bind(profile.latitude.trim())
            .bind(profile.longitude.trim())
            .execute(&self.pool)
            .await?;
        self.user_profile(user_id).await
    }

    pub async fn set_hamdb_error(&self, user_id: Uuid, message: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET hamdb_last_error=$2 WHERE id=$1")
            .bind(user_id)
            .bind(message.chars().take(240).collect::<String>())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_member_profile(
        &self,
        user_id: Uuid,
        display_name: &str,
        global_role: &str,
    ) -> Result<Option<CurrentUser>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(716736629)")
            .execute(&mut *tx)
            .await?;
        if global_role == "member" {
            let administrators: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM users WHERE global_role = 'administrator'",
            )
            .fetch_one(&mut *tx)
            .await?;
            let target_is_administrator: bool =
                sqlx::query_scalar("SELECT global_role = 'administrator' FROM users WHERE id = $1")
                    .bind(user_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .unwrap_or(false);
            if should_preserve_last_administrator(administrators, target_is_administrator) {
                tx.rollback().await?;
                return Ok(None);
            }
        }
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "UPDATE users SET display_name=$2, global_role=$3 WHERE id=$1 RETURNING id, callsign, display_name, global_role",
        )
        .bind(user_id)
        .bind(display_name)
        .bind(global_role)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row.map(|r| CurrentUser {
            id: r.0,
            callsign: r.1,
            display_name: r.2,
            global_role: r.3,
        }))
    }

    pub async fn update_user_display_name(
        &self,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<Option<CurrentUser>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, String)>(
            "UPDATE users SET display_name=$2 WHERE id=$1 RETURNING id, callsign, display_name, global_role",
        )
        .bind(user_id)
        .bind(display_name)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|r| CurrentUser { id: r.0, callsign: r.1, display_name: r.2, global_role: r.3 }))
    }

    pub async fn activity_summary(
        &self,
        user_id: Uuid,
        scope: &str,
        scope_id: Option<Uuid>,
        period_days: i32,
    ) -> Result<qsonaut_protocol::ActivitySummary, sqlx::Error> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, Option<DateTime<Utc>>)>(
            "SELECT COUNT(*)::bigint, COUNT(DISTINCT q.callsign)::bigint, COUNT(DISTINCT q.band)::bigint, COUNT(DISTINCT q.mode)::bigint, COALESCE(SUM(q.points),0)::bigint, MAX(q.occurred_at) FROM qso_logs q WHERE q.user_id=$1 AND ($2=0 OR q.occurred_at >= now() - make_interval(days => $2)) AND ($3='overall' OR ($3='club' AND EXISTS (SELECT 1 FROM events e WHERE e.id=q.event_id AND e.club_id=$4)) OR ($3='contest' AND q.event_id=$4))",
        )
        .bind(user_id)
        .bind(period_days.clamp(0, 3650))
        .bind(scope)
        .bind(scope_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(qsonaut_protocol::ActivitySummary {
            scope: scope.to_owned(),
            scope_id,
            period_days: period_days.clamp(0, 3650),
            qso_count: row.0,
            unique_callsigns: row.1,
            band_count: row.2,
            mode_count: row.3,
            points: row.4,
            last_qso_at: row.5,
            status: if row.0 > 0 {
                "active".to_owned()
            } else {
                "quiet".to_owned()
            },
        })
    }

    pub async fn activity_visibility_policies(
        &self,
        user_id: Uuid,
        is_administrator: bool,
    ) -> Result<Vec<ActivityVisibility>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, Uuid, String, Uuid, bool, DateTime<Utc>)>(
            "SELECT p.id, CASE WHEN p.identity_id IS NOT NULL THEN 'identity' WHEN p.club_id IS NOT NULL THEN 'club' ELSE 'event' END AS scope, COALESCE(p.identity_id,p.club_id,p.event_id) AS scope_id, p.visibility, p.updated_by_user_id, ($2 OR (p.identity_id IS NOT NULL AND (c.owner_user_id=$1 OR EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=c.club_id AND m.user_id=$1 AND m.membership_status='active' AND m.role IN ('owner','coordinator')))) OR (p.club_id IS NOT NULL AND EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=p.club_id AND m.user_id=$1 AND m.membership_status='active' AND m.role IN ('owner','coordinator'))) OR (p.event_id IS NOT NULL AND EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=et.club_id AND m.user_id=$1 AND m.membership_status='active' AND m.role IN ('owner','coordinator')))) AS can_edit, p.updated_at FROM activity_visibility_policies p LEFT JOIN managed_callsigns c ON c.id=p.identity_id LEFT JOIN events et ON et.id=p.event_id WHERE $2 OR (p.identity_id IS NOT NULL AND (c.owner_user_id=$1 OR EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=c.club_id AND m.user_id=$1 AND m.membership_status='active'))) OR (p.club_id IS NOT NULL AND EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=p.club_id AND m.user_id=$1 AND m.membership_status='active')) OR (p.event_id IS NOT NULL AND EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=et.club_id AND m.user_id=$1 AND m.membership_status='active')) ORDER BY scope,scope_id",
        )
        .bind(user_id)
        .bind(is_administrator)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| ActivityVisibility { id: row.0, scope: row.1, scope_id: row.2, visibility: row.3, updated_by_user_id: row.4, can_edit: row.5, updated_at: row.6 })
                .collect()
        })
    }

    pub async fn set_activity_visibility(
        &self,
        user_id: Uuid,
        input: &ActivityVisibilityInput,
    ) -> Result<ActivityVisibility, sqlx::Error> {
        let identity_id = (input.scope == "identity").then_some(input.scope_id);
        let club_id = (input.scope == "club").then_some(input.scope_id);
        let event_id = (input.scope == "event").then_some(input.scope_id);
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO activity_visibility_policies (id,identity_id,club_id,event_id,visibility,updated_by_user_id) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(identity_id)
        .bind(club_id)
        .bind(event_id)
        .bind(input.visibility.trim())
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        sqlx::query_as::<_, (Uuid, String, Uuid, String, Uuid, DateTime<Utc>)>(
            "UPDATE activity_visibility_policies SET visibility=$1,updated_by_user_id=$2,updated_at=now() WHERE ($3::uuid IS NOT NULL AND identity_id=$3) OR ($4::uuid IS NOT NULL AND club_id=$4) OR ($5::uuid IS NOT NULL AND event_id=$5) RETURNING id,CASE WHEN identity_id IS NOT NULL THEN 'identity' WHEN club_id IS NOT NULL THEN 'club' ELSE 'event' END AS scope,COALESCE(identity_id,club_id,event_id) AS scope_id,visibility,updated_by_user_id,updated_at",
        )
        .bind(input.visibility.trim())
        .bind(user_id)
        .bind(identity_id)
        .bind(club_id)
        .bind(event_id)
        .fetch_one(&self.pool)
        .await
        .map(|row| ActivityVisibility { id: row.0, scope: row.1, scope_id: row.2, visibility: row.3, updated_by_user_id: row.4, can_edit: true, updated_at: row.5 })
    }

    pub async fn update_member_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE users SET password_hash=$2 WHERE id=$1")
            .bind(user_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            > 0;
        if updated {
            sqlx::query("DELETE FROM sessions WHERE user_id=$1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM device_tokens WHERE user_id=$1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(updated)
    }

    pub async fn member_memberships(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MemberClubRole>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, Option<String>, String)>(
            "SELECT c.id, c.name, c.callsign, cm.role FROM club_members cm JOIN clubs c ON c.id=cm.club_id WHERE cm.user_id=$1 ORDER BY c.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| MemberClubRole {
                    club_id: r.0,
                    club_name: r.1,
                    club_callsign: r.2,
                    role: r.3,
                })
                .collect()
        })
    }

    pub async fn create_member(
        &self,
        callsign: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<CurrentUser, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>("INSERT INTO users (id, callsign, display_name, password_hash, global_role) VALUES ($1,$2,$3,$4,'member') RETURNING id, callsign, display_name, global_role")
            .bind(Uuid::new_v4()).bind(callsign).bind(display_name).bind(password_hash).fetch_one(&self.pool).await?;
        Ok(CurrentUser {
            id: row.0,
            callsign: row.1,
            display_name: row.2,
            global_role: row.3,
        })
    }

    pub async fn club_members(&self, club_id: Uuid) -> Result<Vec<ClubMembership>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, String, Option<String>, Option<NaiveDate>, DateTime<Utc>)>("SELECT cm.club_id, u.id, u.callsign, u.display_name, cm.role, cm.membership_status, cm.dues_status, cm.membership_number, cm.renewal_due_on, cm.created_at FROM club_members cm JOIN users u ON u.id=cm.user_id WHERE cm.club_id=$1 ORDER BY u.callsign")
            .bind(club_id).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(|r| ClubMembership { club_id: r.0, user_id: r.1, callsign: r.2, display_name: r.3, role: r.4, membership_status: r.5, dues_status: r.6, membership_number: r.7, renewal_due_on: r.8, joined_at: r.9 }).collect())
    }

    pub async fn set_club_member(
        &self,
        club_id: Uuid,
        user_id: Uuid,
        input: &ClubMembershipInput,
    ) -> Result<ClubMembership, sqlx::Error> {
        sqlx::query("INSERT INTO club_members (club_id,user_id,role,membership_status,dues_status,membership_number,renewal_due_on) VALUES ($1,$2,$3,COALESCE($4,'active'),COALESCE($5,'not_tracked'),$6,$7) ON CONFLICT (club_id,user_id) DO UPDATE SET role=excluded.role,membership_status=COALESCE($4,club_members.membership_status),dues_status=COALESCE($5,club_members.dues_status),membership_number=COALESCE($6,club_members.membership_number),renewal_due_on=COALESCE($7,club_members.renewal_due_on),updated_at=now()")
            .bind(club_id).bind(user_id).bind(&input.role).bind(input.membership_status.as_deref()).bind(input.dues_status.as_deref()).bind(input.membership_number.as_deref()).bind(input.renewal_due_on).execute(&self.pool).await?;
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, String, Option<String>, Option<NaiveDate>, DateTime<Utc>)>("SELECT cm.club_id, u.id, u.callsign, u.display_name, cm.role, cm.membership_status, cm.dues_status, cm.membership_number, cm.renewal_due_on, cm.created_at FROM club_members cm JOIN users u ON u.id=cm.user_id WHERE cm.club_id=$1 AND cm.user_id=$2")
            .bind(club_id).bind(user_id).fetch_one(&self.pool).await?;
        Ok(ClubMembership {
            club_id: row.0,
            user_id: row.1,
            callsign: row.2,
            display_name: row.3,
            role: row.4,
            membership_status: row.5,
            dues_status: row.6,
            membership_number: row.7,
            renewal_due_on: row.8,
            joined_at: row.9,
        })
    }

    pub async fn club_role(
        &self,
        club_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT role FROM club_members WHERE club_id=$1 AND user_id=$2")
            .bind(club_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn active_owner_count(&self, club_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM club_members WHERE club_id=$1 AND role='owner' AND membership_status='active'")
            .bind(club_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn request_club_join(
        &self,
        club_id: Uuid,
        user_id: Uuid,
    ) -> Result<ClubJoinRequest, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, ClubJoinRequestRow>("WITH inserted AS (INSERT INTO club_join_requests (id,club_id,user_id) SELECT $1,$2,$3 WHERE NOT EXISTS (SELECT 1 FROM club_members WHERE club_id=$2 AND user_id=$3) RETURNING id,club_id,user_id,status,requested_at,reviewed_at,reviewed_by) SELECT r.id,r.club_id,r.user_id,u.callsign,u.display_name,r.status,r.requested_at,r.reviewed_at,r.reviewed_by FROM inserted r JOIN users u ON u.id=r.user_id")
            .bind(id).bind(club_id).bind(user_id).fetch_one(&self.pool).await.map(ClubJoinRequest::from)
    }

    pub async fn club_join_requests(
        &self,
        club_id: Uuid,
    ) -> Result<Vec<ClubJoinRequest>, sqlx::Error> {
        sqlx::query_as::<_, ClubJoinRequestRow>("SELECT r.id,r.club_id,r.user_id,u.callsign,u.display_name,r.status,r.requested_at,r.reviewed_at,r.reviewed_by FROM club_join_requests r JOIN users u ON u.id=r.user_id WHERE r.club_id=$1 ORDER BY CASE WHEN r.status='pending' THEN 0 ELSE 1 END,r.requested_at DESC")
            .bind(club_id).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(ClubJoinRequest::from).collect())
    }

    pub async fn review_club_join_request(
        &self,
        club_id: Uuid,
        request_id: Uuid,
        reviewer_id: Uuid,
        decision: &str,
        role: &str,
    ) -> Result<Option<ClubJoinRequest>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let request = sqlx::query_as::<_, (Uuid,)>("SELECT user_id FROM club_join_requests WHERE id=$1 AND club_id=$2 AND status='pending' FOR UPDATE")
            .bind(request_id).bind(club_id).fetch_optional(&mut *tx).await?;
        let Some((user_id,)) = request else {
            tx.rollback().await?;
            return Ok(None);
        };
        if decision == "approved" {
            sqlx::query("INSERT INTO club_members (club_id,user_id,role) VALUES ($1,$2,$3) ON CONFLICT (club_id,user_id) DO NOTHING")
                .bind(club_id).bind(user_id).bind(role).execute(&mut *tx).await?;
        }
        let row = sqlx::query_as::<_, ClubJoinRequestRow>("WITH reviewed AS (UPDATE club_join_requests SET status=$3,reviewed_at=now(),reviewed_by=$4 WHERE id=$1 AND club_id=$2 RETURNING id,club_id,user_id,status,requested_at,reviewed_at,reviewed_by) SELECT r.id,r.club_id,r.user_id,u.callsign,u.display_name,r.status,r.requested_at,r.reviewed_at,r.reviewed_by FROM reviewed r JOIN users u ON u.id=r.user_id")
            .bind(request_id).bind(club_id).bind(decision).bind(reviewer_id).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(ClubJoinRequest::from(row)))
    }

    pub async fn remove_club_member(
        &self,
        club_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query("DELETE FROM club_members WHERE club_id=$1 AND user_id=$2")
            .bind(club_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn session_user(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<CurrentUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT u.id, u.callsign, u.display_name, u.global_role FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = $1 AND s.expires_at > now()"
        ).bind(token_hash).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| CurrentUser {
            id: r.0,
            callsign: r.1,
            display_name: r.2,
            global_role: r.3,
        }))
    }

    pub async fn delete_session(&self, token_hash: &[u8]) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_device_token(
        &self,
        user_id: Uuid,
        device_name: &str,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        scopes: &Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO device_tokens (id, user_id, device_name, token_hash, expires_at, scopes) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(device_name.trim())
            .bind(token_hash)
            .bind(expires_at)
            .bind(scopes)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_device_authorization_grant(
        &self,
        input: &DeviceAuthorizationGrantInput<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "WITH expired AS (DELETE FROM device_authorization_grants WHERE expires_at <= now())
             INSERT INTO device_authorization_grants
                (id,client_id,device_name,client_version,device_code_hash,user_code_hash,expires_at,interval_seconds)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(input.id)
        .bind(input.client_id.trim())
        .bind(input.device_name.trim())
        .bind(input.client_version.trim())
        .bind(input.device_code_hash)
        .bind(input.user_code_hash)
        .bind(input.expires_at)
        .bind(input.interval_seconds)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn device_authorization_for_user_code(
        &self,
        user_code_hash: &[u8],
    ) -> Result<Option<DeviceAuthorizationApprovalRecord>, sqlx::Error> {
        sqlx::query_as::<_, (String, String, String, DateTime<Utc>)>(
            "SELECT device_name,client_id,client_version,expires_at
             FROM device_authorization_grants
             WHERE user_code_hash=$1 AND status='pending' AND expires_at > now()",
        )
        .bind(user_code_hash)
        .fetch_optional(&self.pool)
        .await
        .map(|row| {
            row.map(|(device_name, client_id, client_version, expires_at)| {
                DeviceAuthorizationApprovalRecord {
                    device_name,
                    client_id,
                    client_version,
                    expires_at,
                }
            })
        })
    }

    pub async fn decide_device_authorization(
        &self,
        user_id: Uuid,
        user_code_hash: &[u8],
        approved: bool,
    ) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query(
            "UPDATE device_authorization_grants
             SET status=CASE WHEN $3 THEN 'approved' ELSE 'denied' END,
                 user_id=CASE WHEN $3 THEN $1 ELSE NULL END,
                 approved_at=CASE WHEN $3 THEN now() ELSE NULL END
             WHERE user_code_hash=$2 AND status='pending' AND expires_at > now()",
        )
        .bind(user_id)
        .bind(user_code_hash)
        .bind(approved)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn device_authorization_status(
        &self,
        device_code_hash: &[u8],
        client_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>, i32)>, sqlx::Error> {
        sqlx::query_as::<_, (String, DateTime<Utc>, i32)>(
            "SELECT status,expires_at,interval_seconds
             FROM device_authorization_grants
             WHERE device_code_hash=$1 AND client_id=$2",
        )
        .bind(device_code_hash)
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn issue_device_token_from_authorization(
        &self,
        device_code_hash: &[u8],
        client_id: &str,
        token_id: Uuid,
        token_hash: &[u8],
        token_expires_at: DateTime<Utc>,
        scopes: &Value,
    ) -> Result<Option<(Uuid, String)>, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let Some((user_id, device_name)) = sqlx::query_as::<_, (Uuid, String)>(
            "UPDATE device_authorization_grants
             SET status='consumed', consumed_at=now()
             WHERE device_code_hash=$1 AND client_id=$2 AND status='approved' AND expires_at > now()
             RETURNING user_id,device_name",
        )
        .bind(device_code_hash)
        .bind(client_id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO device_tokens (id,user_id,device_name,token_hash,expires_at,scopes)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(token_id)
        .bind(user_id)
        .bind(device_name.trim())
        .bind(token_hash)
        .bind(token_expires_at)
        .bind(scopes)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(Some((user_id, device_name)))
    }

    pub async fn device_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<qsonaut_protocol::DeviceTokenRecord>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>, DateTime<Utc>)>(
            "SELECT id,device_name,expires_at,last_used_at,created_at FROM device_tokens WHERE user_id=$1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| qsonaut_protocol::DeviceTokenRecord {
                    id: row.0,
                    device_name: row.1,
                    expires_at: row.2,
                    last_used_at: row.3,
                    created_at: row.4,
                })
                .collect()
        })
    }

    pub async fn device_token_name(
        &self,
        user_id: Uuid,
        token_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT device_name FROM device_tokens WHERE id=$1 AND user_id=$2")
            .bind(token_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn delete_device_token_by_id(
        &self,
        user_id: Uuid,
        token_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query("DELETE FROM device_tokens WHERE id=$1 AND user_id=$2")
                .bind(token_id)
                .bind(user_id)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }

    pub async fn device_token_user(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<CurrentUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "UPDATE device_tokens d SET last_used_at=now() FROM users u WHERE d.user_id=u.id AND d.token_hash=$1 AND d.expires_at > now() RETURNING u.id, u.callsign, u.display_name, u.global_role",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| CurrentUser {
            id: r.0,
            callsign: r.1,
            display_name: r.2,
            global_role: r.3,
        }))
    }

    pub async fn device_token_scopes(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        sqlx::query_scalar::<_, Value>(
            "SELECT scopes FROM device_tokens WHERE token_hash=$1 AND expires_at > now()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map(|value| {
            value.map(|scopes| {
                scopes
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
        })
    }

    pub async fn delete_device_token(&self, token_hash: &[u8]) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM device_tokens WHERE token_hash=$1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clubs(&self, viewer_id: Uuid, is_admin: bool) -> Result<Vec<Club>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, Option<String>, String, i64, i64, Option<String>, Option<String>, Option<String>, bool)>(
            "SELECT c.id,c.name,c.callsign,c.description,count(cm.user_id),count(cm.user_id) FILTER (WHERE cm.membership_status='active' AND (cm.dues_status IN ('due','overdue') OR cm.renewal_due_on <= current_date + 30)),mine.role,mine.membership_status,(SELECT status FROM club_join_requests r WHERE r.club_id=c.id AND r.user_id=$1 ORDER BY requested_at DESC LIMIT 1),COALESCE($2 OR (mine.role IN ('owner','coordinator') AND mine.membership_status='active'), false) FROM clubs c LEFT JOIN club_members cm ON cm.club_id=c.id LEFT JOIN club_members mine ON mine.club_id=c.id AND mine.user_id=$1 GROUP BY c.id,mine.role,mine.membership_status ORDER BY c.name",
        )
        .bind(viewer_id)
        .bind(is_admin)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| Club {
                    id: r.0,
                    name: r.1,
                    callsign: r.2,
                    description: r.3,
                    member_count: r.4,
                    renewal_attention_count: r.5,
                    my_role: r.6,
                    my_membership_status: r.7,
                    join_request_status: r.8,
                    can_manage: r.9,
                })
                .collect()
        })
    }

    pub async fn create_club(
        &self,
        input: &ClubInput,
        owner_id: Uuid,
    ) -> Result<Club, sqlx::Error> {
        self.create_club_with_limit(input, owner_id, None)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn create_club_with_limit(
        &self,
        input: &ClubInput,
        owner_id: Uuid,
        max_clubs: Option<i64>,
    ) -> Result<Option<Club>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        if let Some(limit) = max_clubs {
            sqlx::query("SELECT pg_advisory_xact_lock(7265734)")
                .execute(&mut *tx)
                .await?;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM clubs")
                .fetch_one(&mut *tx)
                .await?;
            if count >= limit {
                tx.rollback().await?;
                return Ok(None);
            }
        }
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, (Uuid, String, Option<String>, String)>("INSERT INTO clubs (id, name, callsign, description) VALUES ($1, $2, $3, $4) RETURNING id, name, callsign, description")
            .bind(id).bind(input.name.trim()).bind(input.callsign.as_deref()).bind(input.description.trim()).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO club_members (club_id, user_id, role) VALUES ($1, $2, 'owner')")
            .bind(id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(Club {
            id: row.0,
            name: row.1,
            callsign: row.2,
            description: row.3,
            member_count: 1,
            renewal_attention_count: 0,
            my_role: Some("owner".to_owned()),
            my_membership_status: Some("active".to_owned()),
            join_request_status: None,
            can_manage: true,
        }))
    }

    pub async fn update_club(
        &self,
        club_id: Uuid,
        viewer_id: Uuid,
        input: &ClubInput,
    ) -> Result<Option<Club>, sqlx::Error> {
        let updated = sqlx::query(
            "UPDATE clubs SET name=$3, callsign=$4, description=$5, updated_at=now() WHERE id=$1 AND EXISTS (SELECT 1 FROM club_members WHERE club_id=$1 AND user_id=$2 AND role IN ('owner','coordinator'))",
        )
        .bind(club_id)
        .bind(viewer_id)
        .bind(input.name.trim())
        .bind(input.callsign.as_deref())
        .bind(input.description.trim())
        .execute(&self.pool)
        .await?
        .rows_affected();
        if updated == 0 {
            return Ok(None);
        }
        Ok(self
            .clubs(viewer_id, false)
            .await?
            .into_iter()
            .find(|club| club.id == club_id))
    }

    pub async fn contest_templates(&self) -> Result<Vec<ContestTemplate>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, String, String, String, String, i32, bool, Value, Value, Value, Value)>(
            "SELECT id, contest_type, name, description, organization, icon, rules_url, definition_version, is_builtin, scoring_rules, required_fields, validation_rules, schedule FROM contest_templates WHERE is_active=true ORDER BY organization, name",
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| ContestTemplate {
                    id: r.0,
                    contest_type: r.1,
                    name: r.2,
                    description: r.3,
                    organization: r.4,
                    icon: r.5,
                    rules_url: r.6,
                    definition_version: r.7,
                    is_builtin: r.8,
                    scoring_rules: r.9,
                    required_fields: r.10,
                    validation_rules: r.11,
                    schedule: r.12,
                })
                .collect()
        })
    }

    async fn sync_builtin_contests(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for template in qsonaut_contests::builtin_templates() {
            sqlx::query(
                "INSERT INTO contest_templates (id, contest_type, name, description, organization, icon, rules_url, definition_version, is_builtin, scoring_rules, required_fields, validation_rules, schedule, is_active) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,$9,$10,$11,$12,true) ON CONFLICT (contest_type) DO UPDATE SET name=EXCLUDED.name, description=EXCLUDED.description, organization=EXCLUDED.organization, icon=EXCLUDED.icon, rules_url=EXCLUDED.rules_url, definition_version=EXCLUDED.definition_version, is_builtin=true, scoring_rules=EXCLUDED.scoring_rules, required_fields=EXCLUDED.required_fields, validation_rules=EXCLUDED.validation_rules, schedule=EXCLUDED.schedule, is_active=true, updated_at=now() WHERE contest_templates.is_builtin",
            )
            .bind(template.id)
            .bind(template.contest_type)
            .bind(template.name)
            .bind(template.description)
            .bind(template.organization)
            .bind(template.icon)
            .bind(template.rules_url)
            .bind(template.definition_version)
            .bind(template.scoring_rules)
            .bind(template.required_fields)
            .bind(template.validation_rules)
            .bind(template.schedule)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    pub async fn events(&self) -> Result<Vec<Event>, sqlx::Error> {
        sqlx::query_as::<_, EventRow>("SELECT e.id, e.club_id, e.name, e.contest_name, e.special_callsign, e.starts_at, e.ends_at, e.status, e.contest_template_id, e.contest_definition_version, e.contest_config, (SELECT COUNT(*) FROM event_participants p WHERE p.event_id=e.id)::bigint AS participant_count FROM events e ORDER BY e.starts_at")
            .fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(Event::from).collect())
    }

    pub async fn managed_callsigns_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ManagedCallsign>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, Option<Uuid>, Option<Uuid>, Option<Uuid>, String, DateTime<Utc>, Option<DateTime<Utc>>, String, String)>(
            "SELECT c.id,c.callsign,c.identity_type,c.owner_user_id,c.club_id,c.event_id,c.status,c.effective_from,c.expires_at,c.verification_status,c.authority FROM managed_callsigns c WHERE c.owner_user_id=$1 OR (c.club_id IS NOT NULL AND EXISTS (SELECT 1 FROM club_members m WHERE m.club_id=c.club_id AND m.user_id=$1 AND m.membership_status='active')) ORDER BY c.callsign",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|r| ManagedCallsign {
            id: r.0, callsign: r.1, identity_type: r.2, owner_user_id: r.3, club_id: r.4,
            event_id: r.5, status: r.6, effective_from: r.7, expires_at: r.8,
            verification_status: r.9, authority: r.10,
        }).collect())
    }

    pub async fn managed_callsigns(&self) -> Result<Vec<ManagedCallsign>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, Option<Uuid>, Option<Uuid>, Option<Uuid>, String, DateTime<Utc>, Option<DateTime<Utc>>, String, String)>(
            "SELECT id,callsign,identity_type,owner_user_id,club_id,event_id,status,effective_from,expires_at,verification_status,authority FROM managed_callsigns ORDER BY callsign",
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|r| ManagedCallsign {
            id: r.0, callsign: r.1, identity_type: r.2, owner_user_id: r.3, club_id: r.4,
            event_id: r.5, status: r.6, effective_from: r.7, expires_at: r.8,
            verification_status: r.9, authority: r.10,
        }).collect())
    }

    pub async fn create_special_callsign(
        &self,
        input: &ManagedCallsignInput,
        verified: bool,
        actor_user_id: Uuid,
    ) -> Result<ManagedCallsign, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO managed_callsigns (id,callsign,identity_type,club_id,event_id,status,verification_status,authority,authority_reference,effective_from,expires_at) SELECT $1,upper($2),'special',e.club_id,e.id,$3,$4,$5,$6,e.starts_at,e.ends_at FROM events e WHERE e.id=$7",
        )
        .bind(id)
        .bind(input.callsign.trim())
        .bind(if verified { "active" } else { "pending" })
        .bind(if verified { "verified" } else { "pending" })
        .bind(input.authority.trim())
        .bind(input.authority_reference.trim())
        .bind(input.event_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,'registered',$4)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(actor_user_id)
        .bind(serde_json::json!({
            "verification_status": if verified { "verified" } else { "pending" }
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.managed_callsign(id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn managed_callsign(&self, id: Uuid) -> Result<Option<ManagedCallsign>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String, Option<Uuid>, Option<Uuid>, Option<Uuid>, String, DateTime<Utc>, Option<DateTime<Utc>>, String, String)>(
            "SELECT id,callsign,identity_type,owner_user_id,club_id,event_id,status,effective_from,expires_at,verification_status,authority FROM managed_callsigns WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|r| ManagedCallsign {
            id: r.0, callsign: r.1, identity_type: r.2, owner_user_id: r.3, club_id: r.4,
            event_id: r.5, status: r.6, effective_from: r.7, expires_at: r.8,
            verification_status: r.9, authority: r.10,
        }))
    }

    pub async fn update_special_callsign_status(
        &self,
        id: Uuid,
        status: &str,
        verification_status: &str,
        action: &str,
        actor_user_id: Uuid,
    ) -> Result<ManagedCallsign, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE managed_callsigns SET status=$2, verification_status=$3, updated_at=now() WHERE id=$1 AND identity_type='special' RETURNING id",
        )
        .bind(id)
        .bind(status)
        .bind(verification_status)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
        sqlx::query(
            "INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(updated)
        .bind(actor_user_id)
        .bind(action)
        .bind(serde_json::json!({"status": status, "verification_status": verification_status}))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.managed_callsign(id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn event_participants(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<EventParticipant>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid, Uuid, String, String, String, String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, String, Option<String>, Option<String>)>(
            "SELECT id,event_id,user_id,club_id,callsign_id,operator_callsign,operating_callsign,role,status,starts_at,ends_at,station_label,band,mode FROM event_participants WHERE event_id=$1 ORDER BY operating_callsign,operator_callsign",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|r| EventParticipant {
            id: r.0, event_id: r.1, user_id: r.2, club_id: r.3, callsign_id: r.4,
            operator_callsign: r.5, operating_callsign: r.6, role: r.7, status: r.8,
            starts_at: r.9, ends_at: r.10, station_label: r.11, band: r.12, mode: r.13,
        }).collect())
    }

    pub async fn event_score(&self, event_id: Uuid) -> Result<EventScore, sqlx::Error> {
        sqlx::query_as::<_, (i64, i64, i64, Value)>(
            "SELECT COALESCE(SUM(points),0)::bigint, COUNT(*)::bigint, COUNT(*) FILTER (WHERE is_duplicate)::bigint, COALESCE(jsonb_agg(multipliers) FILTER (WHERE multipliers <> '{}'::jsonb), '[]'::jsonb) FROM qso_logs WHERE event_id=$1",
        )
        .bind(event_id)
        .fetch_one(&self.pool)
        .await
        .map(|row| EventScore {
            event_id,
            total_points: row.0,
            qso_count: row.1,
            duplicate_count: row.2,
            multiplier_values: row.3,
        })
    }

    pub async fn create_event_participant(
        &self,
        event_id: Uuid,
        input: &EventParticipantInput,
        actor_user_id: Option<Uuid>,
    ) -> Result<EventParticipant, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        if let Some(actor_user_id) = actor_user_id {
            sqlx::query("SELECT set_config('qsonaut.actor_user_id',$1,true)")
                .bind(actor_user_id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query(
            "INSERT INTO event_participants (id,event_id,user_id,club_id,callsign_id,operator_callsign,operating_callsign,role,status,starts_at,ends_at,station_label,band,mode) SELECT $1,$2,$3,e.club_id,$4,$5,c.callsign,$6,$7,COALESCE($8,e.starts_at),COALESCE($9,e.ends_at),$10,$11,$12 FROM events e JOIN managed_callsigns c ON c.id=$4 WHERE e.id=$2",
        )
        .bind(id).bind(event_id).bind(input.user_id).bind(input.callsign_id)
        .bind(input.operator_callsign.trim().to_ascii_uppercase()).bind(input.role.trim())
        .bind(input.status.trim()).bind(input.starts_at).bind(input.ends_at)
        .bind(input.station_label.trim()).bind(input.band.as_deref()).bind(input.mode.as_deref())
        .execute(&mut *tx).await?;
        tx.commit().await?;
        self.event_participants(event_id)
            .await?
            .into_iter()
            .find(|participant| participant.id == id)
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update_event_participant(
        &self,
        event_id: Uuid,
        participant_id: Uuid,
        input: &EventParticipantInput,
        actor_user_id: Uuid,
    ) -> Result<Option<EventParticipant>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('qsonaut.actor_user_id',$1,true)")
            .bind(actor_user_id.to_string())
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE event_participants p SET user_id=$3,callsign_id=$4,operator_callsign=upper(trim($5)),operating_callsign=c.callsign,role=trim($6),status=trim($7),starts_at=COALESCE($8,e.starts_at),ends_at=COALESCE($9,e.ends_at),station_label=trim($10),band=$11,mode=$12,updated_at=now() FROM events e,managed_callsigns c WHERE p.id=$1 AND p.event_id=$2 AND e.id=$2 AND c.id=$4 RETURNING p.id",
        )
        .bind(participant_id).bind(event_id).bind(input.user_id).bind(input.callsign_id)
        .bind(input.operator_callsign.trim()).bind(input.role.trim()).bind(input.status.trim())
        .bind(input.starts_at).bind(input.ends_at).bind(input.station_label.trim())
        .bind(input.band.as_deref()).bind(input.mode.as_deref())
        .fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        if updated.is_none() {
            return Ok(None);
        }
        Ok(self
            .event_participants(event_id)
            .await?
            .into_iter()
            .find(|participant| participant.id == participant_id))
    }

    pub async fn create_event(&self, input: &EventInput) -> Result<Event, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, EventRow>("INSERT INTO events (id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_definition_version, contest_config, 0::bigint AS participant_count")
            .bind(id).bind(input.club_id).bind(input.name.trim()).bind(input.contest_name.trim()).bind(input.special_callsign.as_deref().map(str::to_ascii_uppercase)).bind(input.starts_at).bind(input.ends_at).bind(input.status.as_str()).bind(input.contest_template_id).bind(&input.contest_config)
            .fetch_one(&self.pool).await.map(Event::from)
    }

    pub async fn create_managed_event(
        &self,
        input: &EventInput,
        actor_user_id: Uuid,
        verify_special_callsign: bool,
    ) -> Result<Event, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let event = sqlx::query_as::<_, EventRow>("INSERT INTO events (id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_definition_version, contest_config, 0::bigint AS participant_count")
            .bind(id).bind(input.club_id).bind(input.name.trim()).bind(input.contest_name.trim()).bind(input.special_callsign.as_deref().map(str::to_ascii_uppercase)).bind(input.starts_at).bind(input.ends_at).bind(input.status.as_str()).bind(input.contest_template_id).bind(&input.contest_config)
            .fetch_one(&mut *tx).await?;
        if let Some(callsign) = input.special_callsign.as_deref() {
            let identity_id = Uuid::new_v4();
            sqlx::query("INSERT INTO managed_callsigns (id,callsign,identity_type,club_id,event_id,status,verification_status,authority,effective_from,expires_at) VALUES ($1,upper($2),'special',$3,$4,$5,$6,'event-request',$7,$8)")
                .bind(identity_id).bind(callsign.trim()).bind(input.club_id).bind(id)
                .bind(if verify_special_callsign { "active" } else { "pending" })
                .bind(if verify_special_callsign { "verified" } else { "pending" })
                .bind(input.starts_at).bind(input.ends_at).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,'registered',$4)")
                .bind(Uuid::new_v4()).bind(identity_id).bind(actor_user_id)
                .bind(serde_json::json!({"source":"event creation","verification_status":if verify_special_callsign { "verified" } else { "pending" }}))
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(Event::from(event))
    }

    pub async fn set_event_status(
        &self,
        event_id: Uuid,
        status: EventStatus,
    ) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, EventRow>("WITH updated AS (UPDATE events SET status=$2, updated_at=now() WHERE id=$1 RETURNING id,club_id,name,contest_name,special_callsign,starts_at,ends_at,status,contest_template_id,contest_definition_version,contest_config) SELECT u.id,u.club_id,u.name,u.contest_name,u.special_callsign,u.starts_at,u.ends_at,u.status,u.contest_template_id,u.contest_definition_version,u.contest_config,(SELECT COUNT(*) FROM event_participants p WHERE p.event_id=u.id)::bigint AS participant_count FROM updated u")
            .bind(event_id).bind(status.as_str()).fetch_optional(&self.pool).await.map(|row| row.map(Event::from))
    }

    pub async fn update_event(
        &self,
        event_id: Uuid,
        input: &EventUpdateInput,
    ) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, EventRow>("WITH updated AS (UPDATE events SET club_id=$2,name=$3,contest_name=$4,special_callsign=$5,starts_at=$6,ends_at=$7,status=$8,contest_template_id=$9,contest_config=$10,updated_at=now() WHERE id=$1 RETURNING id,club_id,name,contest_name,special_callsign,starts_at,ends_at,status,contest_template_id,contest_definition_version,contest_config) SELECT u.id,u.club_id,u.name,u.contest_name,u.special_callsign,u.starts_at,u.ends_at,u.status,u.contest_template_id,u.contest_definition_version,u.contest_config,(SELECT COUNT(*) FROM event_participants p WHERE p.event_id=u.id)::bigint AS participant_count FROM updated u")
            .bind(event_id).bind(input.club_id).bind(input.name.trim()).bind(input.contest_name.trim()).bind(input.special_callsign.as_deref()).bind(input.starts_at).bind(input.ends_at).bind(input.status.as_str()).bind(input.contest_template_id).bind(&input.contest_config).fetch_optional(&self.pool).await.map(|row| row.map(Event::from))
    }

    pub async fn update_managed_event(
        &self,
        event_id: Uuid,
        input: &EventUpdateInput,
        actor_user_id: Uuid,
        verify_special_callsign: bool,
    ) -> Result<Option<Event>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let event = sqlx::query_as::<_, EventRow>("WITH updated AS (UPDATE events SET club_id=$2,name=$3,contest_name=$4,special_callsign=$5,starts_at=$6,ends_at=$7,status=$8,contest_template_id=$9,contest_config=$10,updated_at=now() WHERE id=$1 RETURNING id,club_id,name,contest_name,special_callsign,starts_at,ends_at,status,contest_template_id,contest_definition_version,contest_config) SELECT u.id,u.club_id,u.name,u.contest_name,u.special_callsign,u.starts_at,u.ends_at,u.status,u.contest_template_id,u.contest_definition_version,u.contest_config,(SELECT COUNT(*) FROM event_participants p WHERE p.event_id=u.id)::bigint AS participant_count FROM updated u")
            .bind(event_id).bind(input.club_id).bind(input.name.trim()).bind(input.contest_name.trim()).bind(input.special_callsign.as_deref().map(str::to_ascii_uppercase)).bind(input.starts_at).bind(input.ends_at).bind(input.status.as_str()).bind(input.contest_template_id).bind(&input.contest_config).fetch_optional(&mut *tx).await?;
        let Some(event) = event else {
            tx.rollback().await?;
            return Ok(None);
        };
        let existing = sqlx::query_as::<_, (Uuid, String)>("SELECT id,callsign FROM managed_callsigns WHERE event_id=$1 AND identity_type='special' FOR UPDATE")
            .bind(event_id).fetch_optional(&mut *tx).await?;
        let special_callsign = input
            .special_callsign
            .as_deref()
            .map(str::to_ascii_uppercase);
        match (existing, special_callsign.as_deref()) {
            (Some((identity_id, old_callsign)), Some(callsign)) if old_callsign != callsign => {
                sqlx::query("UPDATE managed_callsigns SET callsign=upper($2),club_id=$3,status=$4,verification_status=$5,authority='event-request',effective_from=$6,expires_at=$7,updated_at=now() WHERE id=$1")
                    .bind(identity_id).bind(callsign.trim()).bind(input.club_id)
                    .bind(if verify_special_callsign { "active" } else { "pending" })
                    .bind(if verify_special_callsign { "verified" } else { "pending" })
                    .bind(input.starts_at).bind(input.ends_at).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,'changed',$4)")
                    .bind(Uuid::new_v4()).bind(identity_id).bind(actor_user_id)
                    .bind(serde_json::json!({"previous_callsign":old_callsign,"callsign":callsign,"verification_status":if verify_special_callsign { "verified" } else { "pending" }}))
                    .execute(&mut *tx).await?;
            }
            (Some((identity_id, _)), Some(_)) => {
                sqlx::query("UPDATE managed_callsigns SET club_id=$2,effective_from=$3,expires_at=$4,updated_at=now() WHERE id=$1")
                    .bind(identity_id).bind(input.club_id).bind(input.starts_at).bind(input.ends_at)
                    .execute(&mut *tx).await?;
            }
            (Some((identity_id, old_callsign)), None) => {
                sqlx::query(
                    "UPDATE managed_callsigns SET status='revoked',updated_at=now() WHERE id=$1",
                )
                .bind(identity_id)
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,'revoked',$4)")
                    .bind(Uuid::new_v4()).bind(identity_id).bind(actor_user_id)
                    .bind(serde_json::json!({"source":"event update","callsign":old_callsign}))
                    .execute(&mut *tx).await?;
            }
            (None, Some(callsign)) => {
                let identity_id = Uuid::new_v4();
                sqlx::query("INSERT INTO managed_callsigns (id,callsign,identity_type,club_id,event_id,status,verification_status,authority,effective_from,expires_at) VALUES ($1,upper($2),'special',$3,$4,$5,$6,'event-request',$7,$8)")
                    .bind(identity_id).bind(callsign.trim()).bind(input.club_id).bind(event_id)
                    .bind(if verify_special_callsign { "active" } else { "pending" })
                    .bind(if verify_special_callsign { "verified" } else { "pending" })
                    .bind(input.starts_at).bind(input.ends_at).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO managed_callsign_audit (id,callsign_id,actor_user_id,action,details) VALUES ($1,$2,$3,'registered',$4)")
                    .bind(Uuid::new_v4()).bind(identity_id).bind(actor_user_id)
                    .bind(serde_json::json!({"source":"event update","verification_status":if verify_special_callsign { "verified" } else { "pending" }}))
                    .execute(&mut *tx).await?;
            }
            (None, None) => {}
        }
        tx.commit().await?;
        Ok(Some(Event::from(event)))
    }

    pub async fn station_presence(
        &self,
        user_id: Option<Uuid>,
    ) -> Result<Vec<StationPresence>, sqlx::Error> {
        sqlx::query_as::<_, StationPresenceRow>("SELECT sp.id, sp.user_id, u.callsign, u.display_name, sp.instance_id, sp.station_label, sp.radio_manufacturer, sp.radio_model, sp.frequency_hz, sp.band, sp.mode, sp.qsonaut_version, sp.platform, sp.status, sp.metadata, sp.last_seen FROM station_presence sp JOIN users u ON u.id=sp.user_id WHERE ($1::uuid IS NULL OR sp.user_id=$1) ORDER BY sp.last_seen DESC")
            .bind(user_id).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(StationPresence::from).collect())
    }

    pub async fn create_diagnostic_report(
        &self,
        user_id: Uuid,
        input: &DiagnosticReportInput,
    ) -> Result<DiagnosticReport, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String, Uuid, String, String, Value, DateTime<Utc>)>(
            "WITH inserted AS (INSERT INTO diagnostic_reports (id,user_id,instance_id,category,summary,payload) VALUES ($1,$2,$3,$4,$5,$6) RETURNING *) SELECT d.id,d.user_id,u.callsign,d.instance_id,d.category,d.summary,d.payload,d.created_at FROM inserted d JOIN users u ON u.id=d.user_id",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(input.instance_id)
        .bind(input.category.trim())
        .bind(input.summary.trim())
        .bind(&input.payload)
        .fetch_one(&self.pool)
        .await
        .map(|row| DiagnosticReport { id: row.0, user_id: row.1, operator_callsign: row.2, instance_id: row.3, category: row.4, summary: row.5, payload: row.6, created_at: row.7 })
    }

    pub async fn diagnostic_reports(
        &self,
        limit: i64,
    ) -> Result<Vec<DiagnosticReport>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String, Uuid, String, String, Value, DateTime<Utc>)>(
            "SELECT d.id,d.user_id,u.callsign,d.instance_id,d.category,d.summary,d.payload,d.created_at FROM diagnostic_reports d JOIN users u ON u.id=d.user_id ORDER BY d.created_at DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|row| DiagnosticReport { id: row.0, user_id: row.1, operator_callsign: row.2, instance_id: row.3, category: row.4, summary: row.5, payload: row.6, created_at: row.7 }).collect())
    }

    pub async fn diagnostic_reports_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<DiagnosticReport>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String, Uuid, String, String, Value, DateTime<Utc>)>(
            "SELECT d.id,d.user_id,u.callsign,d.instance_id,d.category,d.summary,d.payload,d.created_at FROM diagnostic_reports d JOIN users u ON u.id=d.user_id WHERE d.user_id=$1 ORDER BY d.created_at DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|row| DiagnosticReport { id: row.0, user_id: row.1, operator_callsign: row.2, instance_id: row.3, category: row.4, summary: row.5, payload: row.6, created_at: row.7 }).collect())
    }

    pub async fn purge_diagnostic_reports(&self) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query("DELETE FROM diagnostic_reports")
            .execute(&self.pool)
            .await?
            .rows_affected())
    }

    pub async fn upsert_station_presence(
        &self,
        user_id: Uuid,
        input: &StationPresenceInput,
    ) -> Result<StationPresence, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO station_presence (id,user_id,instance_id,station_label,radio_manufacturer,radio_model,frequency_hz,band,mode,qsonaut_version,platform,status,metadata,last_seen) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,now()) ON CONFLICT (user_id,instance_id) DO UPDATE SET station_label=excluded.station_label,radio_manufacturer=excluded.radio_manufacturer,radio_model=excluded.radio_model,frequency_hz=excluded.frequency_hz,band=excluded.band,mode=excluded.mode,qsonaut_version=excluded.qsonaut_version,platform=excluded.platform,status=excluded.status,metadata=excluded.metadata,last_seen=now()")
            .bind(id).bind(user_id).bind(input.instance_id).bind(input.station_label.trim()).bind(input.radio_manufacturer.as_deref()).bind(input.radio_model.as_deref()).bind(input.frequency_hz).bind(input.band.as_deref()).bind(input.mode.as_deref()).bind(input.qsonaut_version.trim()).bind(input.platform.trim()).bind(input.status.as_str()).bind(&input.metadata).execute(&self.pool).await?;
        self.station_presence(Some(user_id))
            .await?
            .into_iter()
            .find(|station| station.instance_id == input.instance_id)
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn qso_logs_for_viewer(
        &self,
        viewer_id: Uuid,
        is_administrator: bool,
        limit: i64,
    ) -> Result<Vec<QsoLog>, sqlx::Error> {
        let query = qso_log_query("FROM qso_logs q JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id LEFT JOIN managed_callsigns identity ON identity.id=q.callsign_id LEFT JOIN LATERAL (SELECT p.visibility FROM activity_visibility_policies p WHERE p.event_id=q.event_id LIMIT 1) event_policy ON true LEFT JOIN LATERAL (SELECT p.visibility FROM activity_visibility_policies p WHERE p.identity_id=q.callsign_id LIMIT 1) identity_policy ON true LEFT JOIN LATERAL (SELECT p.visibility FROM activity_visibility_policies p WHERE p.club_id=COALESCE(identity.club_id,e.club_id) LIMIT 1) club_policy ON true WHERE $1 OR q.user_id=$2 OR COALESCE(event_policy.visibility,identity_policy.visibility,club_policy.visibility)='global' OR (COALESCE(event_policy.visibility,identity_policy.visibility,club_policy.visibility)='members' AND COALESCE(identity.club_id,e.club_id) IS NOT NULL AND EXISTS (SELECT 1 FROM club_members cm WHERE cm.user_id=$2 AND cm.club_id=COALESCE(identity.club_id,e.club_id) AND cm.membership_status='active')) ORDER BY q.occurred_at DESC LIMIT $3");
        sqlx::query_as::<_, QsoLogRow>(&query)
            .bind(is_administrator)
            .bind(viewer_id)
            .bind(limit.clamp(1, 1000))
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.into_iter().map(QsoLog::from).collect())
    }

    pub async fn activity_map_points(
        &self,
        viewer_id: Uuid,
        is_administrator: bool,
        scope: &str,
        scope_id: Option<Uuid>,
    ) -> Result<Vec<ActivityMapPoint>, sqlx::Error> {
        let mut logs = self.qso_logs_for_viewer(viewer_id, is_administrator, 1_000).await?;
        let identities = self
            .managed_callsigns()
            .await?
            .into_iter()
            .map(|identity| (identity.id, (identity.club_id, identity.owner_user_id)))
            .collect::<HashMap<_, _>>();
        let events = self.events().await?;
        match scope {
            "overall" => logs.retain(|log| {
                log.callsign_id
                    .and_then(|identity_id| identities.get(&identity_id))
                    .is_some_and(|(_, owner_user_id)| *owner_user_id == Some(viewer_id))
            }),
            "identity" => {
                let identity_id = scope_id.ok_or_else(|| {
                    sqlx::Error::Protocol("identity map scope requires a target".into())
                })?;
                logs.retain(|log| log.callsign_id == Some(identity_id));
            }
            "club" => {
                let club_id = scope_id.ok_or_else(|| {
                    sqlx::Error::Protocol("club map scope requires a target".into())
                })?;
                let event_ids = events
                    .iter()
                    .filter(|event| event.club_id == club_id)
                    .map(|event| event.id)
                    .collect::<std::collections::HashSet<_>>();
                logs.retain(|log| {
                    log.event_id.is_some_and(|event_id| event_ids.contains(&event_id))
                        || log
                            .callsign_id
                            .and_then(|identity_id| identities.get(&identity_id))
                            .and_then(|(identity_club_id, _)| *identity_club_id)
                            .is_some_and(|identity_club_id| identity_club_id == club_id)
                });
            }
            "event" => {
                let event_id = scope_id.ok_or_else(|| {
                    sqlx::Error::Protocol("event map scope requires a target".into())
                })?;
                logs.retain(|log| log.event_id == Some(event_id));
            }
            _ => unreachable!("map scope is validated by the API"),
        }
        let mut points = HashMap::<String, ActivityMapPoint>::new();
        for log in logs {
            let grid = grid_from_exchange(&log.exchange);
            let Some(grid) = grid.and_then(maidenhead_center) else {
                continue;
            };
            points
                .entry(grid.0.clone())
                .and_modify(|point| {
                    point.qso_count += 1;
                    point.last_qso_at = point.last_qso_at.max(log.occurred_at);
                })
                .or_insert(ActivityMapPoint {
                    grid: grid.0,
                    latitude: grid.1,
                    longitude: grid.2,
                    qso_count: 1,
                    last_qso_at: log.occurred_at,
                });
        }
        let mut points = points.into_values().collect::<Vec<_>>();
        points.sort_by(|left, right| right.qso_count.cmp(&left.qso_count).then_with(|| right.last_qso_at.cmp(&left.last_qso_at)));
        Ok(points)
    }

    pub async fn create_qso_log(
        &self,
        user_id: Uuid,
        input: &QsoLogInput,
    ) -> Result<QsoLog, sqlx::Error> {
        // Retries are expected when a native client reconnects. Resolve the
        // existing record before inserting so a successful retry is a normal
        // success response rather than a unique-constraint error.
        let existing_query = qso_log_query("FROM qso_logs q JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id WHERE q.user_id=$1 AND q.idempotency_key=$2");
        if let Some(existing) = sqlx::query_as::<_, QsoLogRow>(&existing_query)
            .bind(user_id)
            .bind(input.idempotency_key)
            .fetch_optional(&self.pool)
            .await?
        {
            return Ok(existing.into());
        }
        let id = Uuid::new_v4();
        let insert = sqlx::query("INSERT INTO qso_logs (id,user_id,event_id,operating_callsign,callsign_id,idempotency_key,callsign,band,mode,frequency_hz,occurred_at,rst_sent,rst_received,exchange,points,source) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) ON CONFLICT (user_id,idempotency_key) DO NOTHING")
            .bind(id).bind(user_id).bind(input.event_id).bind(input.operating_callsign.as_deref().map(str::trim).map(str::to_ascii_uppercase)).bind(input.callsign_id).bind(input.idempotency_key).bind(input.callsign.trim().to_ascii_uppercase()).bind(input.band.trim()).bind(input.mode.trim().to_ascii_uppercase()).bind(input.frequency_hz).bind(input.occurred_at).bind(input.rst_sent.as_deref()).bind(input.rst_received.as_deref()).bind(&input.exchange).bind(input.points).bind(input.source.trim()).execute(&self.pool).await?;
        if insert.rows_affected() == 0 {
            return sqlx::query_as::<_, QsoLogRow>(&existing_query)
                .bind(user_id).bind(input.idempotency_key).fetch_one(&self.pool).await.map(Into::into);
        }
        let inserted_query = qso_log_query("FROM qso_logs q JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id WHERE q.id=$1");
        sqlx::query_as::<_, QsoLogRow>(&inserted_query)
            .bind(id).fetch_one(&self.pool).await.map(QsoLog::from)
    }

    pub async fn qso_log_owner(&self, log_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT user_id FROM qso_logs WHERE id=$1")
            .bind(log_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn active_club_member(
        &self,
        user_id: Uuid,
        club_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM club_members WHERE user_id=$1 AND club_id=$2 AND membership_status='active')")
            .bind(user_id)
            .bind(club_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn event_club_id(&self, event_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT club_id FROM events WHERE id=$1")
            .bind(event_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn club_exists(&self, club_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM clubs WHERE id=$1)")
            .bind(club_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn create_qso_share_link(
        &self,
        qso_log_id: Uuid,
        created_by: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO qso_share_links (id,qso_log_id,created_by,token_hash,expires_at) VALUES ($1,$2,$3,$4,$5)")
            .bind(id)
            .bind(qso_log_id)
            .bind(created_by)
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn shared_qso_log(&self, token_hash: &[u8]) -> Result<Option<QsoLog>, sqlx::Error> {
        let query = qso_log_query("FROM qso_share_links s JOIN qso_logs q ON q.id=s.qso_log_id JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at > now()");
        sqlx::query_as::<_, QsoLogRow>(&query)
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(QsoLog::from))
    }

    pub async fn revoke_qso_share_link(
        &self,
        share_id: Uuid,
        user_id: Uuid,
        is_administrator: bool,
    ) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query(
            "UPDATE qso_share_links SET revoked_at=now() WHERE id=$1 AND (created_by=$2 OR $3)",
        )
        .bind(share_id)
        .bind(user_id)
        .bind(is_administrator)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn qso_share_links_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ShareLinkRecord>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>, String, DateTime<Utc>, Option<DateTime<Utc>>, DateTime<Utc>)>(
            "SELECT s.id,s.qso_log_id,q.occurred_at,q.callsign,s.expires_at,s.revoked_at,s.created_at FROM qso_share_links s JOIN qso_logs q ON q.id=s.qso_log_id WHERE s.created_by=$1 ORDER BY s.created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|row| ShareLinkRecord { id: row.0, qso_log_id: row.1, occurred_at: row.2, worked_callsign: row.3, expires_at: row.4, revoked_at: row.5, created_at: row.6 }).collect())
    }

    pub async fn record_audit_event(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        metadata: &Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO audit_events (id,actor_user_id,action,target_type,target_id,metadata) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(Uuid::new_v4()).bind(actor_user_id).bind(action).bind(target_type).bind(target_id).bind(metadata).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn purge_expired_artifacts(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM access_challenges WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM diagnostic_reports WHERE created_at < now() - interval '30 days'")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM qso_share_links WHERE (revoked_at IS NOT NULL AND revoked_at < now() - interval '30 days') OR expires_at < now() - interval '30 days'")
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn channel_messages(&self, limit: i64) -> Result<Vec<ChannelMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChannelMessageRow>("SELECT m.id, m.user_id, u.callsign AS author_callsign, m.event_id, m.channel, m.message, m.metadata, m.created_at FROM channel_messages m JOIN users u ON u.id=m.user_id ORDER BY m.created_at DESC LIMIT $1")
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.into_iter().map(ChannelMessage::from).collect())
    }

    pub async fn create_channel_message(
        &self,
        user_id: Uuid,
        input: &ChannelMessageInput,
    ) -> Result<ChannelMessage, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, ChannelMessageRow>("WITH inserted AS (INSERT INTO channel_messages (id,user_id,event_id,channel,message,metadata) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id,user_id,event_id,channel,message,metadata,created_at) SELECT i.id,i.user_id,u.callsign AS author_callsign,i.event_id,i.channel,i.message,i.metadata,i.created_at FROM inserted i JOIN users u ON u.id=i.user_id")
            .bind(id)
            .bind(user_id)
            .bind(input.event_id)
            .bind(input.channel.trim())
            .bind(input.message.trim())
            .bind(&input.metadata)
            .fetch_one(&self.pool)
            .await
            .map(ChannelMessage::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{grid_from_exchange, maidenhead_center, should_preserve_last_administrator};

    #[test]
    fn protects_the_last_administrator_only_when_downgrading_one() {
        assert!(should_preserve_last_administrator(1, true));
        assert!(!should_preserve_last_administrator(2, true));
        assert!(!should_preserve_last_administrator(1, false));
    }

    #[test]
    fn maidenhead_grid_centres_are_validated_and_stable() {
        let (grid, latitude, longitude) = maidenhead_center("cn87").expect("valid four-character grid");
        assert_eq!(grid, "CN87");
        assert!((latitude - 47.5).abs() < f64::EPSILON);
        assert!((longitude + 123.0).abs() < f64::EPSILON);
        assert!(maidenhead_center("CN87XX").is_some());
        assert!(maidenhead_center("not-a-grid").is_none());
        assert!(maidenhead_center("CN8").is_none());
        assert_eq!(grid_from_exchange(&serde_json::json!({"fields_received":{"grid":"FN42"}})), Some("FN42"));
    }
}
