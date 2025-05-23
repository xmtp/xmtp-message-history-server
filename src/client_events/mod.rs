use base64::{prelude::BASE64_STANDARD, Engine};
use xmtp_db::{client_events::Details, group_intent::IntentKind};

const CARD_HTML: &str = include_str!("_card.html");
const METRIC_HTML: &str = include_str!("_metric.html");
const CHANGE_HTML: &str = include_str!("_change.html");

#[derive(Default)]
struct Card {
    icon: Option<&'static str>,
    title: String,
    metrics: Vec<Metric>,
}

#[derive(Default)]
struct Metric {
    icon: &'static str,
    title: &'static str,
    value: String,
    from: Option<String>,
    extra_info: Option<String>,
}

const TITLE_ICONS: &[(&str, &str)] = &[
    ("ClientBuild", "🏗️"),
    ("MsgStreamConnect", "🚣‍♀️"),
    ("GroupWelcome", "🤗"),
    ("EpochChange", "🔼"),
    ("QueueIntent", "🏁"),
];

pub fn render_event_card(event: String, details: &[u8]) -> String {
    let event = event.replace("\"", "");
    let title = event.to_case(Case::Title).to_uppercase();
    let icon = TITLE_ICONS.iter().find(|(n, _)| n == &event).map(|e| e.1);
    let metrics = if let Ok(details) = serde_json::from_slice::<Details>(details) {
        details_to_metrics(details)
    } else {
        vec![]
    };

    let card = Card {
        icon,
        title,
        metrics,
    };

    let metrics = card
        .metrics
        .into_iter()
        .map(|m| {
            let partial = match m.from {
                Some(from) => CHANGE_HTML.replace(r#"{ from }"#, &from),
                None => String::new(),
            };
            let extra_info = BASE64_STANDARD.encode(m.extra_info.unwrap_or_default());

            METRIC_HTML
                .replace(r#"{ icon }"#, &m.icon)
                .replace(r#"{ label }"#, &m.title)
                .replace(r#"{ value }"#, &m.value)
                .replace(r#"{ extra-info }"#, &extra_info)
                .replace(r#"{ change }"#, &partial)
        })
        .collect::<Vec<_>>()
        .join("");

    CARD_HTML
        .replace("{ icon }", &card.icon.unwrap_or_default())
        .replace("{ title }", &card.title)
        .replace("{ metrics }", &metrics)
}

use convert_case::{Case, Casing};

fn details_to_metrics(details: Details) -> Vec<Metric> {
    match details {
        Details::EpochChange {
            new_epoch,
            prev_epoch,
            validated_commit,
            ..
        } => vec![Metric {
            icon: "E",
            title: "Epoch",
            value: format!("{}", new_epoch),
            from: Some(format!("{}", prev_epoch)),
            extra_info: validated_commit,
        }],
        Details::GroupWelcome {
            conversation_type,
            added_by_inbox_id,
        } => vec![
            Metric {
                icon: "T",
                title: "Conversation Type",
                value: format!("{:?}", conversation_type),
                ..Default::default()
            },
            Metric {
                icon: "👤",
                title: "Added By",
                value: added_by_inbox_id,
                ..Default::default()
            },
        ],
        Details::GroupCreate {
            conversation_type,
            initial_members,
        } => vec![
            Metric {
                icon: "T",
                title: "Conversation Type",
                value: format!("{:?}", conversation_type),
                ..Default::default()
            },
            Metric {
                icon: "👥",
                title: "Initial Members",
                value: format!("{initial_members:?}"),
                ..Default::default()
            },
        ],
        Details::QueueIntent {
            intent_kind: IntentKind::SendMessage,
        } => vec![],
        Details::QueueIntent { intent_kind } => vec![Metric {
            icon: "?",
            title: "Intent Kind",
            value: format!("{intent_kind:?}"),
            ..Default::default()
        }],
        Details::MsgStreamConnect { .. } => vec![],
    }
}
