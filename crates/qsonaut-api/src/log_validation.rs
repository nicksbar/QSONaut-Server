use qsonaut_protocol::QsoLogInput;
use serde_json::Value;

use crate::auth::validate_callsign;

pub(crate) fn validate_log(input: &QsoLogInput) -> Result<(), String> {
    validate_callsign(&input.callsign).map_err(|_| "invalid contact callsign".to_owned())?;
    if input.callsign_id.is_none()
        || input
            .operating_callsign
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err("every QSO requires an operating callsign identity".to_owned());
    }
    validate_callsign(input.operating_callsign.as_deref().unwrap_or_default())
        .map_err(|_| "invalid operating callsign".to_owned())?;
    if input.band.trim().is_empty() || input.band.trim().len() > 40 {
        return Err("band must contain 1 to 40 characters".to_owned());
    }
    if input.mode.trim().is_empty() || input.mode.trim().len() > 40 {
        return Err("mode must contain 1 to 40 characters".to_owned());
    }
    if input.frequency_hz.is_some_and(|frequency| frequency < 0) {
        return Err("frequency cannot be negative".to_owned());
    }
    if !input.exchange.is_object() || input.exchange.to_string().len() > 8_192 {
        return Err("exchange must be an object no larger than 8 KiB".to_owned());
    }
    for section in ["fields_sent", "fields_received"] {
        if input.exchange.get(section).is_some_and(|fields| !fields.is_object()) {
            return Err(format!("exchange {section} must be an object"));
        }
    }
    if input.source.trim().is_empty() || input.source.len() > 40 {
        return Err("log source must contain 1 to 40 characters".to_owned());
    }
    Ok(())
}

pub(crate) fn canonicalize_contest_exchange(exchange: &mut Value) {
    let Some(exchange) = exchange.as_object_mut() else {
        return;
    };
    for section in ["fields_sent", "fields_received"] {
        let Some(fields) = exchange.get_mut(section).and_then(Value::as_object_mut) else {
            continue;
        };
        let keys = fields.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            let canonical = key.to_ascii_lowercase();
            if canonical == key {
                continue;
            }
            if fields.contains_key(&canonical) {
                fields.remove(&key);
            } else if let Some(value) = fields.remove(&key) {
                fields.insert(canonical, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contest_field_keys_are_canonicalized_for_database_rules() {
        let mut exchange = serde_json::json!({
            "fields_sent": { "CLASS": "1A", "section": "WMA" },
            "fields_received": { "SECTION": "EMA" }
        });
        canonicalize_contest_exchange(&mut exchange);
        assert_eq!(exchange["fields_sent"]["class"], "1A");
        assert_eq!(exchange["fields_sent"]["section"], "WMA");
        assert_eq!(exchange["fields_received"]["section"], "EMA");
        assert!(exchange["fields_sent"].get("CLASS").is_none());
    }
}
