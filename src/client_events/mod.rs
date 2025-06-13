use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::Value;

const CARD_HTML: &str = include_str!("_card.html");

pub fn render_event_card(event: &EventSave) -> String {
    let mut details: Value = serde_json::from_slice(&event.details).unwrap();
    let subhead = details
        .as_object_mut()
        .and_then(|d| d.remove(""))
        .and_then(|v| v.as_str().map(|v| v.to_string()))
        .unwrap_or_default();

    let details = serde_json::to_string_pretty(&details).unwrap();
    let details = BASE64_STANDARD.encode(&details);

    let mut icon = "";
    if let Some(icn) = &event.icon {
        icon = icn;
    }

    let bg = format!("level-{:?}", event.level());

    CARD_HTML
        .replace("{ icon }", icon)
        .replace("{ title }", &event.event)
        .replace("{ details }", &details)
        .replace("{ bg }", &bg)
        .replace("{ subhead }", &subhead)
}

use xmtp_proto::xmtp::device_sync::event_backup::EventSave;
