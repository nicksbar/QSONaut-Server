use crate::{
    AppState,
    auth::require_device_with_scopes,
    error::HttpResult,
    log_validation::{canonicalize_contest_exchange, validate_log},
};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
};
use qsonaut_protocol::{
    API_VERSION, ClientEnvelope, ClientMessage, CurrentUser, ServerEnvelope,
    ServerMessage, StationPresenceInput,
};

pub(crate) async fn connect(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> HttpResult<Response> {
    let (user, scopes) = require_device_with_scopes(&state, &headers).await?;
    Ok(upgrade
        .protocols(["qsonaut.v1"])
        .on_upgrade(move |socket| session(socket, state, user, scopes)))
}

async fn session(mut socket: WebSocket, state: AppState, user: CurrentUser, scopes: Vec<String>) {
    let mut channel_messages = state.channel_messages.subscribe();
    if send(
        &mut socket,
        ServerEnvelope {
            protocol_version: API_VERSION.to_owned(),
            event_id: uuid::Uuid::new_v4(),
            message: ServerMessage::Welcome { user: user.clone() },
        },
    )
    .await
    .is_err()
    {
        return;
    }

    loop {
        let message = tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(message)) => message,
                _ => break,
            },
            published = channel_messages.recv() => match published {
                Ok(message) => {
                    if has_scope(&scopes, "messages:read") && send(
                        &mut socket,
                        ServerEnvelope {
                            protocol_version: API_VERSION.to_owned(),
                            event_id: uuid::Uuid::new_v4(),
                            message: ServerMessage::ChannelMessagePublished(message),
                        },
                    )
                    .await
                    .is_err()
                    {
                        break;
                    }
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        };
        let Message::Text(text) = message else {
            if matches!(message, Message::Close(_)) {
                break;
            }
            continue;
        };
        let Ok(envelope) = serde_json::from_str::<ClientEnvelope>(&text) else {
            let _ = send_error(
                &mut socket,
                uuid::Uuid::new_v4(),
                "invalid message envelope",
            )
            .await;
            continue;
        };
        if envelope.protocol_version != API_VERSION {
            let _ = send_error(
                &mut socket,
                envelope.event_id,
                "unsupported protocol version",
            )
            .await;
            continue;
        }
        let response = handle_message(&state, &user, &scopes, envelope.message).await;
        let message = match response {
            Ok(message) => message,
            Err(error) => ServerMessage::Error { message: error },
        };
        if send(
            &mut socket,
            ServerEnvelope {
                protocol_version: API_VERSION.to_owned(),
                event_id: envelope.event_id,
                message,
            },
        )
        .await
        .is_err()
        {
            break;
        }
    }
}

async fn handle_message(
    state: &AppState,
    user: &CurrentUser,
    scopes: &[String],
    message: ClientMessage,
) -> Result<ServerMessage, String> {
    match message {
        ClientMessage::Hello { .. } => Ok(ServerMessage::Ack),
        ClientMessage::Sync => {
            require_scope(scopes, "events:read")?;
            sync_snapshot(state, user, has_scope(scopes, "messages:read")).await
        }
        ClientMessage::Presence(input) => {
            require_scope(scopes, "presence:write")?;
            validate_presence(&input)?;
            state
                .store
                .upsert_station_presence(user.id, &input)
                .await
                .map(ServerMessage::PresenceAccepted)
                .map_err(|error| internal_error(&error))
        }
        ClientMessage::Log(input) => {
            require_scope(scopes, "logs:write")?;
            let mut input = input;
            validate_log(&input)?;
            canonicalize_contest_exchange(&mut input.exchange);
            let qso = state
                .store
                .create_qso_log(user.id, &input)
                .await
                .map_err(|error| log_submission_error(&error))?;
            let event_score = match qso.event_id {
                Some(event_id) => Some(
                    state
                        .store
                        .event_score(event_id)
                        .await
                        .map_err(|error| internal_error(&error))?,
                ),
                None => None,
            };
            Ok(ServerMessage::LogAccepted {
                qso: Box::new(qso),
                event_score,
            })
        }
        ClientMessage::Diagnostic(input) => {
            require_scope(scopes, "diagnostics:write")?;
            if input.category.trim().is_empty() || input.category.len() > 40 {
                return Err("diagnostic category must contain 1 to 40 characters".to_owned());
            }
            if input.summary.trim().is_empty() || input.summary.len() > 240 {
                return Err("diagnostic summary must contain 1 to 240 characters".to_owned());
            }
            if !input.payload.is_object() || input.payload.to_string().len() > 65_536 {
                return Err("diagnostic payload must be an object no larger than 64 KiB".to_owned());
            }
            state
                .store
                .create_diagnostic_report(user.id, &input)
                .await
                .map(ServerMessage::DiagnosticAccepted)
                .map_err(|error| internal_error(&error))
        }
        ClientMessage::ChannelMessage(input) => {
            require_scope(scopes, "messages:write")?;
            validate_channel_message(&input)?;
            let message = state
                .store
                .create_channel_message(user.id, &input)
                .await
                .map_err(|error| internal_error(&error))?;
            let _ = state.channel_messages.send(message.clone());
            Ok(ServerMessage::ChannelMessageAccepted(message))
        }
        ClientMessage::Ping => Ok(ServerMessage::Pong),
    }
}

async fn sync_snapshot(
    state: &AppState,
    user: &CurrentUser,
    include_messages: bool,
) -> Result<ServerMessage, String> {
    let mut events = state
        .store
        .events()
        .await
        .map_err(|error| internal_error(&error))?;
    let contest_templates = state
        .store
        .contest_templates()
        .await
        .map_err(|error| internal_error(&error))?;
    let channel_messages = if include_messages {
        state
            .store
            .channel_messages(200)
            .await
            .map_err(|error| internal_error(&error))?
    } else {
        Vec::new()
    };
    let identities = state
        .store
        .managed_callsigns_for_user(user.id)
        .await
        .map_err(|error| internal_error(&error))?;
    let mut clubs = state
        .store
        .clubs(user.id, false)
        .await
        .map_err(|error| internal_error(&error))?;
    clubs.retain(|club| club.my_role.is_some());
    let club_ids = clubs
        .iter()
        .map(|club| club.id)
        .collect::<std::collections::HashSet<_>>();
    events.retain(|event| club_ids.contains(&event.club_id));
    let mut participants = Vec::new();
    for event in &events {
        participants.extend(
            state
                .store
                .event_participants(event.id)
                .await
                .map_err(|error| internal_error(&error))?
                .into_iter()
                .filter(|participant| participant.user_id == user.id),
        );
    }
    let mut event_scores = Vec::with_capacity(events.len());
    for event in &events {
        event_scores.push(
            state
                .store
                .event_score(event.id)
                .await
                .map_err(|error| internal_error(&error))?,
        );
    }
    Ok(ServerMessage::Snapshot {
        events,
        contest_templates,
        event_scores,
        identities,
        participants,
        channel_messages,
        clubs,
    })
}

fn has_scope(scopes: &[String], required: &str) -> bool {
    scopes.iter().any(|scope| scope == required)
}

fn require_scope(scopes: &[String], required: &str) -> Result<(), String> {
    has_scope(scopes, required)
        .then_some(())
        .ok_or_else(|| format!("device token lacks required scope: {required}"))
}

fn log_submission_error(error: &sqlx::Error) -> String {
    if let sqlx::Error::Database(db) = error
        && db.code().as_deref() == Some("P1001")
    {
        return db.message().to_owned();
    }
    if matches!(error, sqlx::Error::Database(db) if db.is_unique_violation()) {
        "log event was already accepted".to_owned()
    } else {
        internal_error(error)
    }
}

fn validate_channel_message(input: &qsonaut_protocol::ChannelMessageInput) -> Result<(), String> {
    if input.channel.trim().is_empty() || input.channel.trim().len() > 80 {
        return Err("channel must contain 1 to 80 characters".to_owned());
    }
    if input.message.trim().is_empty() || input.message.trim().len() > 2_000 {
        return Err("message must contain 1 to 2000 characters".to_owned());
    }
    if !input.metadata.is_object() || input.metadata.to_string().len() > 8_192 {
        return Err("message metadata must be an object no larger than 8 KiB".to_owned());
    }
    Ok(())
}

fn validate_presence(input: &StationPresenceInput) -> Result<(), String> {
    if !["online", "idle", "offline"].contains(&input.status.as_str()) {
        return Err("invalid station status".to_owned());
    }
    if input.qsonaut_version.trim().is_empty() || input.platform.trim().is_empty() {
        return Err("QSONaut version and platform are required".to_owned());
    }
    if input.frequency_hz.is_some_and(|frequency| frequency < 0) {
        return Err("frequency cannot be negative".to_owned());
    }
    if !input.metadata.is_object() || input.metadata.to_string().len() > 8_192 {
        return Err("station metadata must be an object no larger than 8 KiB".to_owned());
    }
    Ok(())
}


async fn send(socket: &mut WebSocket, envelope: ServerEnvelope) -> Result<(), axum::Error> {
    let text = serde_json::to_string(&envelope).map_err(axum::Error::new)?;
    socket.send(Message::Text(text.into())).await
}

async fn send_error(
    socket: &mut WebSocket,
    event_id: uuid::Uuid,
    message: &str,
) -> Result<(), axum::Error> {
    send(
        socket,
        ServerEnvelope {
            protocol_version: API_VERSION.to_owned(),
            event_id,
            message: ServerMessage::Error {
                message: message.to_owned(),
            },
        },
    )
    .await
}

fn internal_error(error: &sqlx::Error) -> String {
    tracing::error!(%error, "realtime database request failed");
    "server could not process the event".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qsonaut_protocol::QsoLogInput;

    #[test]
    fn websocket_envelope_is_compatible_with_native_client_contract() {
        let envelope = ServerEnvelope {
            protocol_version: API_VERSION.to_owned(),
            event_id: uuid::Uuid::nil(),
            message: ServerMessage::Pong,
        };
        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(value["protocol_version"], "v1");
        assert_eq!(value["type"], "pong");
    }

    #[test]
    fn presence_metadata_remains_bounded() {
        let mut input = StationPresenceInput {
            instance_id: uuid::Uuid::nil(),
            station_label: String::new(),
            radio_manufacturer: None,
            radio_model: None,
            frequency_hz: None,
            band: None,
            mode: None,
            qsonaut_version: "0.2.2".to_owned(),
            platform: "linux-x86_64".to_owned(),
            status: "online".to_owned(),
            metadata: serde_json::json!({}),
        };
        assert!(validate_presence(&input).is_ok());
        input.metadata = serde_json::json!({ "data": "x".repeat(8_192) });
        assert!(validate_presence(&input).is_err());
    }

    #[test]
    fn shared_channel_messages_are_bounded() {
        let valid = qsonaut_protocol::ChannelMessageInput {
            event_id: None,
            channel: "ops".to_owned(),
            message: "Band opening on 20m".to_owned(),
            metadata: serde_json::json!({}),
        };
        assert!(validate_channel_message(&valid).is_ok());

        let mut invalid = valid;
        invalid.message = "x".repeat(2_001);
        assert_eq!(
            validate_channel_message(&invalid).unwrap_err(),
            "message must contain 1 to 2000 characters"
        );
    }

    #[test]
    fn device_scopes_allow_only_the_requested_operation() {
        let scopes = vec!["logs:write".to_owned()];
        assert!(require_scope(&scopes, "logs:write").is_ok());
        assert_eq!(
            require_scope(&scopes, "diagnostics:write").unwrap_err(),
            "device token lacks required scope: diagnostics:write"
        );
    }

    #[test]
    fn log_validation_rejects_malformed_records() {
        let mut input = QsoLogInput {
            event_id: None,
            operating_callsign: None,
            callsign_id: None,
            idempotency_key: uuid::Uuid::new_v4(),
            callsign: "W1AW".to_owned(),
            band: "20m".to_owned(),
            mode: "FT8".to_owned(),
            frequency_hz: Some(14_074_000),
            occurred_at: chrono::Utc::now(),
            rst_sent: None,
            rst_received: None,
            exchange: serde_json::json!({}),
            points: 1,
            source: "qsonaut".to_owned(),
        };
        assert_eq!(
            validate_log(&input).unwrap_err(),
            "every QSO requires an operating callsign identity"
        );
        input.callsign_id = Some(uuid::Uuid::new_v4());
        input.operating_callsign = Some("N7UF".to_owned());
        assert!(validate_log(&input).is_ok());
        input.callsign = "not a callsign".to_owned();
        assert!(validate_log(&input).is_err());
        input.callsign = "W1AW".to_owned();
        input.exchange = serde_json::json!({ "payload": "x".repeat(8_193) });
        assert!(validate_log(&input).is_err());
    }

    #[test]
    fn log_validation_requires_identity_and_object_shaped_exchange_for_events() {
        let mut input = QsoLogInput {
            event_id: Some(uuid::Uuid::new_v4()),
            operating_callsign: None,
            callsign_id: None,
            idempotency_key: uuid::Uuid::new_v4(),
            callsign: "W1AW".to_owned(),
            band: "20m".to_owned(),
            mode: "FT8".to_owned(),
            frequency_hz: None,
            occurred_at: chrono::Utc::now(),
            rst_sent: None,
            rst_received: None,
            exchange: serde_json::json!({}),
            points: 0,
            source: "qsonaut".to_owned(),
        };
        assert_eq!(validate_log(&input).unwrap_err(), "every QSO requires an operating callsign identity");
        input.callsign_id = Some(uuid::Uuid::new_v4());
        input.operating_callsign = Some("W1CLUB".to_owned());
        input.exchange = serde_json::json!({ "fields_received": "not-an-object" });
        assert_eq!(validate_log(&input).unwrap_err(), "exchange fields_received must be an object");
    }
}
