use base64::{prelude::BASE64_STANDARD, Engine};
use xmtp_db::{
    client_events::{ClientEvent, EvtQueueIntent},
    group_intent::IntentKind,
};

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

pub fn render_event_card(event: ClientEvent) -> String {
    let card: Card = event.into();
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

impl From<ClientEvent> for Card {
    fn from(evt: ClientEvent) -> Self {
        match evt {
            ClientEvent::Generic(title) => Self {
                icon: Some("G"),
                title,
                ..Default::default()
            },
            ClientEvent::ClientBuild => Self {
                icon: Some("🏗️"),
                title: "Client Build".to_string(),
                ..Default::default()
            },
            ClientEvent::EpochChange(change) => Self {
                icon: Some("📈"),
                title: "Epoch Change".to_string(),
                metrics: vec![Metric {
                    icon: "E",
                    title: "Epoch",
                    value: format!("{}", change.new_epoch),
                    from: Some(format!("{}", change.prev_epoch)),
                    extra_info: change.validated_commit,
                }],
            },
            ClientEvent::GroupWelcome(welcome) => Self {
                icon: Some("🤗"),
                title: "Group Welcome".to_string(),
                metrics: vec![
                    Metric {
                        icon: "T",
                        title: "Conversation Type",
                        value: format!("{:?}", welcome.conversation_type),
                        ..Default::default()
                    },
                    Metric {
                        icon: "👤",
                        title: "Added By",
                        value: welcome.added_by_inbox_id,
                        ..Default::default()
                    },
                ],
            },
            ClientEvent::QueueIntent(EvtQueueIntent {
                intent_kind: IntentKind::SendMessage,
            }) => Self {
                icon: Some("💬"),
                title: format!("Send Message Intent Queued"),
                metrics: vec![],
            },
            ClientEvent::QueueIntent(EvtQueueIntent { intent_kind }) => Self {
                icon: Some("🏁"),
                title: format!("Queue Intent"),
                metrics: vec![Metric {
                    icon: "?",
                    title: "Intent Kind",
                    value: format!("{intent_kind:?}"),
                    ..Default::default()
                }],
            },
        }
    }
}
