use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::Value;

const CARD_HTML: &str = include_str!("_card.html");

pub fn render_event_card(event: &EventSave) -> String {
    let details: Value = serde_json::from_slice(&event.details).unwrap();
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
}

use xmtp_proto::xmtp::device_sync::event_backup::EventSave;
