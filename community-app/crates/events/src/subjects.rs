use uuid::Uuid;

pub fn message_created(org_id: Uuid, channel_id: Uuid) -> String {
    format!("org.{org_id}.channel.{channel_id}.message.created")
}

pub fn message_updated(org_id: Uuid, channel_id: Uuid) -> String {
    format!("org.{org_id}.channel.{channel_id}.message.updated")
}

pub fn message_deleted(org_id: Uuid, channel_id: Uuid) -> String {
    format!("org.{org_id}.channel.{channel_id}.message.deleted")
}

pub fn notification_created(org_id: Uuid, user_id: Uuid) -> String {
    format!("org.{org_id}.user.{user_id}.notification.created")
}

pub fn media_joined(org_id: Uuid, room_id: Uuid) -> String {
    format!("org.{org_id}.media.{room_id}.joined")
}

pub fn media_left(org_id: Uuid, room_id: Uuid) -> String {
    format!("org.{org_id}.media.{room_id}.left")
}

// Typed media lifecycle subjects (scoped by org + room + session).
// Pattern: org.{org_id}.media.room.{room_id}.session.{session_id}.{event}

pub fn media_session_started(org_id: Uuid, room_id: Uuid, session_id: Uuid) -> String {
    format!("org.{org_id}.media.room.{room_id}.session.{session_id}.started")
}

pub fn media_session_ended(org_id: Uuid, room_id: Uuid, session_id: Uuid) -> String {
    format!("org.{org_id}.media.room.{room_id}.session.{session_id}.ended")
}

pub fn media_participant_joined(org_id: Uuid, room_id: Uuid, session_id: Uuid) -> String {
    format!("org.{org_id}.media.room.{room_id}.session.{session_id}.participant.joined")
}

pub fn media_participant_left(org_id: Uuid, room_id: Uuid, session_id: Uuid) -> String {
    format!("org.{org_id}.media.room.{room_id}.session.{session_id}.participant.left")
}

pub fn media_participant_updated(org_id: Uuid, room_id: Uuid, session_id: Uuid) -> String {
    format!("org.{org_id}.media.room.{room_id}.session.{session_id}.participant.updated")
}

// Messaging (threads + pins)

pub fn thread_reply_created(org_id: Uuid, channel_id: Uuid, thread_id: Uuid) -> String {
    format!("org.{org_id}.channel.{channel_id}.thread.{thread_id}.reply.created")
}

pub fn channel_pins_changed(org_id: Uuid, channel_id: Uuid) -> String {
    format!("org.{org_id}.channel.{channel_id}.pins.changed")
}
