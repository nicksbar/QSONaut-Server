//! `PostgreSQL` persistence for `QSONaut` Server.
#![allow(clippy::missing_errors_doc)]

use chrono::{DateTime, Utc};
use qsonaut_protocol::{
    ChannelMessage, ChannelMessageInput, Club, ClubInput, ClubMembership, ContestTemplate,
    CurrentUser, Event, EventInput, EventStatus, MemberClubRole, QsoLog, QsoLogInput,
    StationPresence, StationPresenceInput,
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
        sqlx::query_as::<_, (Uuid, Uuid, String, String, String)>("SELECT cm.club_id, u.id, u.callsign, u.display_name, cm.role FROM club_members cm JOIN users u ON u.id=cm.user_id WHERE cm.club_id=$1 ORDER BY u.callsign")
            .bind(club_id).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(|r| ClubMembership { club_id: r.0, user_id: r.1, callsign: r.2, display_name: r.3, role: r.4 }).collect())
    }

    pub async fn set_club_member(
        &self,
        club_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ClubMembership, sqlx::Error> {
        sqlx::query("INSERT INTO club_members (club_id,user_id,role) VALUES ($1,$2,$3) ON CONFLICT (club_id,user_id) DO UPDATE SET role=excluded.role")
            .bind(club_id).bind(user_id).bind(role).execute(&self.pool).await?;
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, String)>("SELECT cm.club_id, u.id, u.callsign, u.display_name, cm.role FROM club_members cm JOIN users u ON u.id=cm.user_id WHERE cm.club_id=$1 AND cm.user_id=$2")
            .bind(club_id).bind(user_id).fetch_one(&self.pool).await?;
        Ok(ClubMembership {
            club_id: row.0,
            user_id: row.1,
            callsign: row.2,
            display_name: row.3,
            role: row.4,
        })
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

    pub async fn clubs(&self) -> Result<Vec<Club>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, Option<String>, String)>(
            "SELECT id, name, callsign, description FROM clubs ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| Club {
                    id: r.0,
                    name: r.1,
                    callsign: r.2,
                    description: r.3,
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
