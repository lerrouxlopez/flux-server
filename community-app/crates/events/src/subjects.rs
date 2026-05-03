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

