use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    Work,
    Play,
}

impl NotificationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationMode::Work => "work",
            NotificationMode::Play => "play",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRule {
    MessageAll,
    MessageMentions,
    ThreadReplies,
    PinChanges,
    MediaEvents,
}

impl NotificationRule {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationRule::MessageAll => "message_all",
            NotificationRule::MessageMentions => "message_mentions",
            NotificationRule::ThreadReplies => "thread_replies",
            NotificationRule::PinChanges => "pin_changes",
            NotificationRule::MediaEvents => "media_events",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBehavior {
    pub message_all: bool,
    pub message_mentions: bool,
    pub thread_replies: bool,
    pub pin_changes: bool,
    pub media_events: bool,
}

impl Default for NotificationBehavior {
    fn default() -> Self {
        // Platform default fallback (safe / non-spammy).
        Self {
            message_all: false,
            message_mentions: true,
            thread_replies: false,
            pin_changes: false,
            media_events: false,
        }
    }
}

impl NotificationBehavior {
    pub fn from_rules(rules: &HashMap<String, bool>) -> Self {
        let get = |k: &str| rules.get(k).copied().unwrap_or(false);
        Self {
            message_all: get("message_all"),
            message_mentions: get("message_mentions"),
            thread_replies: get("thread_replies"),
            pin_changes: get("pin_changes"),
            media_events: get("media_events"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedNotificationProfile {
    pub profile_id: Option<Uuid>,
    pub source: String,
    pub behavior: NotificationBehavior,
}

