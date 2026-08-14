use crate::{
    AppState,
    auth::{require_device, validate_callsign},
    error::HttpResult,
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
    API_VERSION, ClientEnvelope, ClientMessage, CurrentUser, QsoLogInput, ServerEnvelope,
    ServerMessage, StationPresenceInput,
};

pub(crate) async fn connect(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> HttpResult<Response> {
    let user = require_device(&state, &headers).await?;
    Ok(upgrade
        .protocols(["qsonaut.v1"])
        .on_upgrade(move |socket| session(socket, state, user)))
}

async fn session(mut socket: WebSocket, state: AppState, user: CurrentUser) {
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
                    if send(
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
        let response = handle_message(&state, &user, envelope.message).await;
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
    message: ClientMessage,
) -> Result<ServerMessage, String> {
    match message {
        ClientMessage::Hello { .. } => Ok(ServerMessage::Ack),
        ClientMessage::Sync => {
            let events = state
                .store
                .events()
                .await
                .map_err(|error| internal_error(&error))?;
            let contest_templates = state
                .store
                .contest_templates()
                .await
                .map_err(|error| internal_error(&error))?;
            let channel_messages = state
                .store
                .channel_messages(200)
                .await
                .map_err(|error| internal_error(&error))?;
            Ok(ServerMessage::Snapshot {
                events,
                contest_templates,
                channel_messages,
            })
        }
        ClientMessage::Presence(input) => {
            validate_presence(&input)?;
            state
                .store
                .upsert_station_presence(user.id, &input)
                .await
                .map(ServerMessage::PresenceAccepted)
                .map_err(|error| internal_error(&error))
        }
        ClientMessage::Log(input) => {
            validate_log(&input)?;
            state
                .store
                .create_qso_log(user.id, &input)
                .await
                .map(ServerMessage::LogAccepted)
                .map_err(|error| {
                    if matches!(&error, sqlx::Error::Database(db) if db.is_unique_violation()) {
                        "log event was already accepted".to_owned()
                    } else {
                        internal_error(&error)
                    }
                })
        }
        ClientMessage::ChannelMessage(input) => {
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

fn validate_log(input: &QsoLogInput) -> Result<(), String> {
    validate_callsign(&input.callsign).map_err(|_| "invalid contact callsign".to_owned())?;
    if input.band.trim().is_empty() || input.mode.trim().is_empty() {
        return Err("callsign, band, and mode are required".to_owned());
    }
    if input.frequency_hz.is_some_and(|frequency| frequency < 0) {
        return Err("frequency cannot be negative".to_owned());
    }
    if !input.exchange.is_object() || input.exchange.to_string().len() > 8_192 {
        return Err("exchange must be an object no larger than 8 KiB".to_owned());
    }
    if input.source.trim().is_empty() || input.source.len() > 40 {
        return Err("log source must contain 1 to 40 characters".to_owned());
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
}
