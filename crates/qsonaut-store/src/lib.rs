//! `PostgreSQL` persistence for `QSONaut` Server.
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, NaiveDate, Utc};
use qsonaut_protocol::{
    ChannelMessage, ChannelMessageInput, Club, ClubElection, ClubElectionInput, ClubGovernance,
    ClubInput, ClubJoinRequest, ClubMembership, ClubMembershipInput, ClubPosition,
    ClubPositionAssignment, ClubPositionAssignmentInput, ClubPositionInput, ContestTemplate,
    CurrentUser, DiagnosticReport, DiagnosticReportInput, Event, EventInput, EventStatus,
    MemberClubRole, QsoLog, QsoLogInput, StationPresence, StationPresenceInput,
};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

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
    contest_config: Value,
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
            contest_config: row.contest_config,
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

#[derive(sqlx::FromRow)]
struct ClubElectionRow {
    id: Uuid,
    club_id: Uuid,
    title: String,
    election_year: i32,
    status: String,
    opens_at: Option<DateTime<Utc>>,
    closes_at: Option<DateTime<Utc>>,
    notes: String,
    position_ids: Vec<Uuid>,
}

impl From<ClubElectionRow> for ClubElection {
    fn from(row: ClubElectionRow) -> Self {
        Self {
            id: row.id,
            club_id: row.club_id,
            title: row.title,
            election_year: row.election_year,
            status: row.status,
            opens_at: row.opens_at,
            closes_at: row.closes_at,
            notes: row.notes,
            position_ids: row.position_ids,
        }
    }
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

    pub async fn update_member_display_name(
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
        .map(|row| {
            row.map(|r| CurrentUser {
                id: r.0,
                callsign: r.1,
                display_name: r.2,
                global_role: r.3,
            })
        })
    }

    pub async fn update_member_global_role(
        &self,
        user_id: Uuid,
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
            let target_is_administrator: bool = sqlx::query_scalar(
                "SELECT global_role = 'administrator' FROM users WHERE id = $1",
            )
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(false);
            if target_is_administrator && administrators <= 1 {
                tx.rollback().await?;
                return Ok(None);
            }
        }
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "UPDATE users SET global_role=$2 WHERE id=$1 RETURNING id, callsign, display_name, global_role",
        )
        .bind(user_id)
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
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO device_tokens (id, user_id, device_name, token_hash, expires_at) VALUES ($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(device_name.trim())
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
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

    pub async fn delete_device_token(&self, token_hash: &[u8]) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM device_tokens WHERE token_hash=$1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clubs(&self, viewer_id: Uuid, is_admin: bool) -> Result<Vec<Club>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, Option<String>, String, i64, i64, Option<String>, Option<String>, bool)>(
            "SELECT c.id,c.name,c.callsign,c.description,count(cm.user_id),count(cm.user_id) FILTER (WHERE cm.membership_status='active' AND (cm.dues_status IN ('due','overdue') OR cm.renewal_due_on <= current_date + 30)),mine.role,(SELECT status FROM club_join_requests r WHERE r.club_id=c.id AND r.user_id=$1 ORDER BY requested_at DESC LIMIT 1),($2 OR mine.role IN ('owner','coordinator')) FROM clubs c LEFT JOIN club_members cm ON cm.club_id=c.id LEFT JOIN club_members mine ON mine.club_id=c.id AND mine.user_id=$1 GROUP BY c.id,mine.role ORDER BY c.name",
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
                    join_request_status: r.7,
                    can_manage: r.8,
                })
                .collect()
        })
    }

    pub async fn create_club(
        &self,
        input: &ClubInput,
        owner_id: Uuid,
    ) -> Result<Club, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, (Uuid, String, Option<String>, String)>("INSERT INTO clubs (id, name, callsign, description) VALUES ($1, $2, $3, $4) RETURNING id, name, callsign, description")
            .bind(id).bind(input.name.trim()).bind(input.callsign.as_deref()).bind(input.description.trim()).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO club_members (club_id, user_id, role) VALUES ($1, $2, 'owner')")
            .bind(id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Club {
            id: row.0,
            name: row.1,
            callsign: row.2,
            description: row.3,
            member_count: 1,
            renewal_attention_count: 0,
            my_role: Some("owner".to_owned()),
            join_request_status: None,
            can_manage: true,
        })
    }

    pub async fn club_governance(&self, club_id: Uuid) -> Result<ClubGovernance, sqlx::Error> {
        let positions = sqlx::query_as::<_, (Uuid, Uuid, String, String, i32, i32, String, String)>("SELECT id,club_id,name,position_type,seats,term_years,election_parity,description FROM club_positions WHERE club_id=$1 ORDER BY position_type,name")
            .bind(club_id).fetch_all(&self.pool).await?.into_iter().map(|r| ClubPosition { id:r.0, club_id:r.1, name:r.2, position_type:r.3, seats:r.4, term_years:r.5, election_parity:r.6, description:r.7 }).collect();
        let assignments = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, i32, NaiveDate, NaiveDate, String)>("SELECT a.id,a.position_id,a.user_id,u.callsign,u.display_name,a.seat_number,a.starts_on,a.ends_on,a.selection_method FROM club_position_assignments a JOIN club_positions p ON p.id=a.position_id JOIN users u ON u.id=a.user_id WHERE p.club_id=$1 ORDER BY a.ends_on DESC,p.name,a.seat_number")
            .bind(club_id).fetch_all(&self.pool).await?.into_iter().map(|r| ClubPositionAssignment { id:r.0, position_id:r.1, user_id:r.2, callsign:r.3, display_name:r.4, seat_number:r.5, starts_on:r.6, ends_on:r.7, selection_method:r.8 }).collect();
        let elections = sqlx::query_as::<_, ClubElectionRow>("SELECT e.id,e.club_id,e.title,e.election_year,e.status,e.opens_at,e.closes_at,e.notes,COALESCE(array_agg(ep.position_id) FILTER (WHERE ep.position_id IS NOT NULL),'{}') AS position_ids FROM club_elections e LEFT JOIN club_election_positions ep ON ep.election_id=e.id WHERE e.club_id=$1 GROUP BY e.id ORDER BY e.election_year DESC,e.created_at DESC")
            .bind(club_id).fetch_all(&self.pool).await?.into_iter().map(ClubElection::from).collect();
        Ok(ClubGovernance {
            positions,
            assignments,
            elections,
        })
    }

    pub async fn create_club_position(
        &self,
        club_id: Uuid,
        input: &ClubPositionInput,
    ) -> Result<ClubPosition, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, i32, i32, String, String)>("INSERT INTO club_positions (id,club_id,name,position_type,seats,term_years,election_parity,description) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id,club_id,name,position_type,seats,term_years,election_parity,description")
            .bind(Uuid::new_v4()).bind(club_id).bind(input.name.trim()).bind(&input.position_type).bind(input.seats).bind(input.term_years).bind(&input.election_parity).bind(input.description.trim()).fetch_one(&self.pool).await?;
        Ok(ClubPosition {
            id: row.0,
            club_id: row.1,
            name: row.2,
            position_type: row.3,
            seats: row.4,
            term_years: row.5,
            election_parity: row.6,
            description: row.7,
        })
    }

    pub async fn assign_club_position(
        &self,
        club_id: Uuid,
        input: &ClubPositionAssignmentInput,
    ) -> Result<ClubPositionAssignment, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, i32, NaiveDate, NaiveDate, String)>("WITH inserted AS (INSERT INTO club_position_assignments (id,position_id,user_id,seat_number,starts_on,ends_on,selection_method) SELECT $1,p.id,$3,$4,$5,$6,$7 FROM club_positions p JOIN club_members cm ON cm.club_id=p.club_id AND cm.user_id=$3 WHERE p.id=$2 AND p.club_id=$8 AND cm.membership_status='active' AND $4 <= p.seats AND NOT EXISTS (SELECT 1 FROM club_position_assignments existing WHERE existing.position_id=p.id AND existing.seat_number=$4 AND daterange(existing.starts_on,existing.ends_on,'[]') && daterange($5,$6,'[]')) RETURNING id,position_id,user_id,seat_number,starts_on,ends_on,selection_method) SELECT a.id,a.position_id,a.user_id,u.callsign,u.display_name,a.seat_number,a.starts_on,a.ends_on,a.selection_method FROM inserted a JOIN users u ON u.id=a.user_id")
            .bind(Uuid::new_v4()).bind(input.position_id).bind(input.user_id).bind(input.seat_number).bind(input.starts_on).bind(input.ends_on).bind(&input.selection_method).bind(club_id).fetch_one(&self.pool).await?;
        Ok(ClubPositionAssignment {
            id: row.0,
            position_id: row.1,
            user_id: row.2,
            callsign: row.3,
            display_name: row.4,
            seat_number: row.5,
            starts_on: row.6,
            ends_on: row.7,
            selection_method: row.8,
        })
    }

    pub async fn create_club_election(
        &self,
        club_id: Uuid,
        creator_id: Uuid,
        input: &ClubElectionInput,
    ) -> Result<ClubElection, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO club_elections (id,club_id,title,election_year,status,opens_at,closes_at,notes,created_by) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(id).bind(club_id).bind(input.title.trim()).bind(input.election_year).bind(&input.status).bind(input.opens_at).bind(input.closes_at).bind(input.notes.trim()).bind(creator_id).execute(&mut *tx).await?;
        for position_id in &input.position_ids {
            let inserted = sqlx::query("INSERT INTO club_election_positions (election_id,position_id) SELECT $1,id FROM club_positions WHERE id=$2 AND club_id=$3")
                .bind(id).bind(position_id).bind(club_id).execute(&mut *tx).await?;
            if inserted.rows_affected() == 0 {
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            }
        }
        tx.commit().await?;
        Ok(ClubElection {
            id,
            club_id,
            title: input.title.trim().to_owned(),
            election_year: input.election_year,
            status: input.status.clone(),
            opens_at: input.opens_at,
            closes_at: input.closes_at,
            notes: input.notes.trim().to_owned(),
            position_ids: input.position_ids.clone(),
        })
    }

    pub async fn set_club_election_status(
        &self,
        club_id: Uuid,
        election_id: Uuid,
        status: &str,
    ) -> Result<Option<ClubElection>, sqlx::Error> {
        let updated = sqlx::query("UPDATE club_elections SET status=$3 WHERE id=$1 AND club_id=$2")
            .bind(election_id)
            .bind(club_id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        if updated.rows_affected() == 0 {
            return Ok(None);
        }
        self.club_governance(club_id).await.map(|governance| {
            governance
                .elections
                .into_iter()
                .find(|election| election.id == election_id)
        })
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
        sqlx::query_as::<_, EventRow>("SELECT id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config FROM events ORDER BY starts_at")
            .fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(Event::from).collect())
    }

    pub async fn create_event(&self, input: &EventInput) -> Result<Event, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, EventRow>("INSERT INTO events (id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config")
            .bind(id).bind(input.club_id).bind(input.name.trim()).bind(input.contest_name.trim()).bind(input.special_callsign.as_deref()).bind(input.starts_at).bind(input.ends_at).bind(input.status.as_str()).bind(input.contest_template_id).bind(&input.contest_config)
            .fetch_one(&self.pool).await.map(Event::from)
    }

    pub async fn set_event_status(
        &self,
        event_id: Uuid,
        status: EventStatus,
    ) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, EventRow>("UPDATE events SET status=$2, updated_at=now() WHERE id=$1 RETURNING id, club_id, name, contest_name, special_callsign, starts_at, ends_at, status, contest_template_id, contest_config")
            .bind(event_id).bind(status.as_str()).fetch_optional(&self.pool).await.map(|row| row.map(Event::from))
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

    pub async fn qso_logs(&self, limit: i64) -> Result<Vec<QsoLog>, sqlx::Error> {
        sqlx::query_as::<_, QsoLogRow>("SELECT q.id, q.user_id, u.callsign AS operator_callsign, q.event_id, e.name AS event_name, q.idempotency_key, q.callsign, q.band, q.mode, q.frequency_hz, q.occurred_at, q.rst_sent, q.rst_received, q.exchange, q.points, q.source FROM qso_logs q JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id ORDER BY q.occurred_at DESC LIMIT $1")
            .bind(limit).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(QsoLog::from).collect())
    }

    pub async fn create_qso_log(
        &self,
        user_id: Uuid,
        input: &QsoLogInput,
    ) -> Result<QsoLog, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO qso_logs (id,user_id,event_id,idempotency_key,callsign,band,mode,frequency_hz,occurred_at,rst_sent,rst_received,exchange,points,source) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(id).bind(user_id).bind(input.event_id).bind(input.idempotency_key).bind(input.callsign.trim().to_ascii_uppercase()).bind(input.band.trim()).bind(input.mode.trim().to_ascii_uppercase()).bind(input.frequency_hz).bind(input.occurred_at).bind(input.rst_sent.as_deref()).bind(input.rst_received.as_deref()).bind(&input.exchange).bind(input.points).bind(input.source.trim()).execute(&self.pool).await?;
        sqlx::query_as::<_, QsoLogRow>("SELECT q.id, q.user_id, u.callsign AS operator_callsign, q.event_id, e.name AS event_name, q.idempotency_key, q.callsign, q.band, q.mode, q.frequency_hz, q.occurred_at, q.rst_sent, q.rst_received, q.exchange, q.points, q.source FROM qso_logs q JOIN users u ON u.id=q.user_id LEFT JOIN events e ON e.id=q.event_id WHERE q.id=$1")
            .bind(id).fetch_one(&self.pool).await.map(QsoLog::from)
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
