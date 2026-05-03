use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    pub event_id: Uuid,
    pub event_type: String,
    pub organization_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub occurred_at: OffsetDateTime,
    pub data: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        event_type: impl Into<String>,
        organization_id: Uuid,
        actor_id: Option<Uuid>,
        data: T,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            event_type: event_type.into(),
            organization_id,
            actor_id,
            occurred_at: OffsetDateTime::now_utc(),
            data,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EventEnvelope<U> {
        EventEnvelope {
            event_id: self.event_id,
            event_type: self.event_type,
            organization_id: self.organization_id,
            actor_id: self.actor_id,
            occurred_at: self.occurred_at,
            data: f(self.data),
        }
    }
}
