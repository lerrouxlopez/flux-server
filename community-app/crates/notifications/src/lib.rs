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
    MentionChannel,
    FriendRequest,
    PinChanges,
    Reaction,
    DirectMessage,
    MediaEvents,
}

impl NotificationRule {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationRule::MessageAll => "message_all",
            NotificationRule::MessageMentions => "message_mentions",
            NotificationRule::ThreadReplies => "thread_replies",
            NotificationRule::MentionChannel => "mention_channel",
            NotificationRule::FriendRequest => "friend_request",
            NotificationRule::PinChanges => "pin_changes",
            NotificationRule::Reaction => "reaction",
            NotificationRule::DirectMessage => "direct_message",
            NotificationRule::MediaEvents => "media_events",
        }
    }

    pub fn all() -> [NotificationRule; 9] {
        [
            NotificationRule::MessageAll,
            NotificationRule::MessageMentions,
            NotificationRule::ThreadReplies,
            NotificationRule::MentionChannel,
            NotificationRule::FriendRequest,
            NotificationRule::PinChanges,
            NotificationRule::Reaction,
            NotificationRule::DirectMessage,
            NotificationRule::MediaEvents,
        ]
    }
}

/// Whether a rule notifies in-app, via desktop push, and/or with a sound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleChannels {
    pub in_app: bool,
    pub desktop: bool,
    pub sound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBehavior {
    pub message_all: RuleChannels,
    pub message_mentions: RuleChannels,
    pub thread_replies: RuleChannels,
    pub mention_channel: RuleChannels,
    pub friend_request: RuleChannels,
    pub pin_changes: RuleChannels,
    pub reaction: RuleChannels,
    pub direct_message: RuleChannels,
    pub media_events: RuleChannels,
}

impl Default for NotificationBehavior {
    fn default() -> Self {
        // Platform default fallback (safe / non-spammy).
        Self {
            message_all: RuleChannels::default(),
            message_mentions: RuleChannels { in_app: true, desktop: true, sound: true },
            thread_replies: RuleChannels::default(),
            mention_channel: RuleChannels::default(),
            friend_request: RuleChannels::default(),
            pin_changes: RuleChannels::default(),
            reaction: RuleChannels::default(),
            direct_message: RuleChannels::default(),
            media_events: RuleChannels::default(),
        }
    }
}

impl NotificationBehavior {
    pub fn from_rows(rows: &HashMap<String, RuleChannels>) -> Self {
        let get = |k: &str| rows.get(k).copied().unwrap_or_default();
        Self {
            message_all: get("message_all"),
            message_mentions: get("message_mentions"),
            thread_replies: get("thread_replies"),
            mention_channel: get("mention_channel"),
            friend_request: get("friend_request"),
            pin_changes: get("pin_changes"),
            reaction: get("reaction"),
            direct_message: get("direct_message"),
            media_events: get("media_events"),
        }
    }
}

/// `from`/`to` are "HH:MM" strings (already formatted by the caller) so the
/// wire shape matches an `<input type="time">` value exactly — the backend
/// still stores/binds these as `time::Time` for the SQL column.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub from: Option<String>,
    pub to: Option<String>,
    pub priority_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedNotificationProfile {
    pub profile_id: Option<Uuid>,
    pub profile_created_by: Option<Uuid>,
    pub source: String,
    pub behavior: NotificationBehavior,
}
