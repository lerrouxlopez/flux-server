use crate::{
    models::auth::ApiError,
    repositories::{MembershipRepository, RoleRepository},
};
use serde::Serialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use tokio::sync::OnceCell;
use uuid::Uuid;

#[derive(Clone)]
pub struct MessagesService {
    pool: PgPool,
    nats_url: String,
    nats: OnceCell<async_nats::Client>,
    memberships: MembershipRepository,
    roles: RoleRepository,
}

#[derive(Serialize)]
struct EventEnvelope {
    event_id: String,
    r#type: String,
    organization_id: String,
    channel_id: String,
    actor_id: String,
    occurred_at: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct MessageView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub body: Option<String>,
    pub kind: String,
    pub created_at: OffsetDateTime,
    pub edited_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub struct ListResult {
    pub messages: Vec<MessageView>,
    pub next_cursor: Option<String>,
}

impl MessagesService {
    pub fn new(
        pool: PgPool,
        nats_url: String,
        memberships: MembershipRepository,
        roles: RoleRepository,
    ) -> Self {
        Self {
            pool,
            nats_url,
            nats: OnceCell::const_new(),
            memberships,
            roles,
        }
    }

    pub async fn list_messages(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        limit: i64,
        before_id: Option<Uuid>,
        before_ts: Option<OffsetDateTime>,
    ) -> Result<ListResult, ApiError> {
        let limit = limit.clamp(1, 100);

        let (organization_id, boundary) = self
            .channel_org_and_boundary(channel_id, before_id)
            .await?;

        self.require(user_id, organization_id, "channels.view").await?;

        let rows = if let Some(ts) = before_ts {
            sqlx::query(
                r#"
                select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
                from messages
                where channel_id = $1
                  and created_at < $2
                order by created_at desc, id desc
                limit $3
                "#,
            )
            .bind(channel_id)
            .bind(ts)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        } else if let Some((created_at, id)) = boundary {
            sqlx::query(
                r#"
                select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
                from messages
                where channel_id = $1
                  and (created_at, id) < ($2, $3)
                order by created_at desc, id desc
                limit $4
                "#,
            )
            .bind(channel_id)
            .bind(created_at)
            .bind(id)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
                from messages
                where channel_id = $1
                order by created_at desc, id desc
                limit $2
                "#,
            )
            .bind(channel_id)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|_| ApiError::internal())?;

        let mut messages: Vec<MessageView> = rows
            .into_iter()
            .map(|row| MessageView {
                id: row.try_get("id").unwrap(),
                organization_id: row.try_get("organization_id").unwrap(),
                channel_id: row.try_get("channel_id").unwrap(),
                sender_id: row.try_get("sender_id").unwrap(),
                body: row.try_get("body").unwrap(),
                kind: row.try_get("kind").unwrap(),
                created_at: row.try_get("created_at").unwrap(),
                edited_at: row.try_get("edited_at").unwrap(),
                deleted_at: row.try_get("deleted_at").unwrap(),
            })
            .collect();

        let next_cursor = if messages.len() as i64 > limit {
            let last = messages.pop().unwrap();
            Some(last.id.to_string())
        } else {
            None
        };

        Ok(ListResult { messages, next_cursor })
    }

    pub async fn create_message(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        body: Option<String>,
    ) -> Result<MessageView, ApiError> {
        let (organization_id, _) = self.channel_org_and_boundary(channel_id, None).await?;

        self.require(user_id, organization_id, "messages.send").await?;

        let now = OffsetDateTime::now_utc();
        let message_id = Uuid::now_v7();

        let mut tx = self.pool.begin().await.map_err(|_| ApiError::internal())?;
        let row = sqlx::query(
            r#"
            insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at)
            values ($1, $2, $3, $4, $5, 'text', $6)
            returning id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
            "#,
        )
        .bind(message_id)
        .bind(organization_id)
        .bind(channel_id)
        .bind(user_id)
        .bind(body)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?;

        tx.commit().await.map_err(|_| ApiError::internal())?;

        let msg = MessageView {
            id: row.try_get("id").unwrap(),
            organization_id: row.try_get("organization_id").unwrap(),
            channel_id: row.try_get("channel_id").unwrap(),
            sender_id: row.try_get("sender_id").unwrap(),
            body: row.try_get("body").unwrap(),
            kind: row.try_get("kind").unwrap(),
            created_at: row.try_get("created_at").unwrap(),
            edited_at: row.try_get("edited_at").unwrap(),
            deleted_at: row.try_get("deleted_at").unwrap(),
        };

        let subject = format!("org.{}.channel.{}.message.created", organization_id, channel_id);
        let evt = EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            r#type: "message.created".to_string(),
            organization_id: organization_id.to_string(),
            channel_id: channel_id.to_string(),
            actor_id: user_id.to_string(),
            occurred_at: now.format(&time::format_description::well_known::Rfc3339).unwrap(),
            data: json!({ "message_id": msg.id }),
        };

        let payload = serde_json::to_vec(&evt).map_err(|_| ApiError::internal())?;
        self.nats()
            .await?
            .publish(subject, payload.into())
            .await
            .map_err(|_| ApiError::internal())?;

        Ok(msg)
    }

    pub async fn update_message(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        body: String,
    ) -> Result<MessageView, ApiError> {
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query(
            r#"
            select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
            from messages
            where id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::not_found)?;

        let organization_id: Uuid = row.try_get("organization_id").map_err(|_| ApiError::internal())?;
        let channel_id: Uuid = row.try_get("channel_id").map_err(|_| ApiError::internal())?;
        let sender_id: Uuid = row.try_get("sender_id").map_err(|_| ApiError::internal())?;

        if sender_id != user_id {
            return Err(ApiError::forbidden());
        }
        self.require(user_id, organization_id, "messages.edit_own").await?;

        let updated = sqlx::query(
            r#"
            update messages
            set body = $2,
                edited_at = $3
            where id = $1 and deleted_at is null
            returning id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
            "#,
        )
        .bind(message_id)
        .bind(body)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::not_found)?;

        let msg = MessageView {
            id: updated.try_get("id").unwrap(),
            organization_id: updated.try_get("organization_id").unwrap(),
            channel_id: updated.try_get("channel_id").unwrap(),
            sender_id: updated.try_get("sender_id").unwrap(),
            body: updated.try_get("body").unwrap(),
            kind: updated.try_get("kind").unwrap(),
            created_at: updated.try_get("created_at").unwrap(),
            edited_at: updated.try_get("edited_at").unwrap(),
            deleted_at: updated.try_get("deleted_at").unwrap(),
        };

        self.publish_event(
            organization_id,
            channel_id,
            user_id,
            "message.updated",
            "message.updated",
            json!({ "message_id": message_id }),
            now,
        )
        .await?;

        Ok(msg)
    }

    pub async fn delete_message(&self, user_id: Uuid, message_id: Uuid) -> Result<(), ApiError> {
        let now = OffsetDateTime::now_utc();
        let row = sqlx::query(
            r#"
            select organization_id, channel_id, sender_id
            from messages
            where id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::not_found)?;

        let organization_id: Uuid = row.try_get("organization_id").map_err(|_| ApiError::internal())?;
        let channel_id: Uuid = row.try_get("channel_id").map_err(|_| ApiError::internal())?;
        let sender_id: Uuid = row.try_get("sender_id").map_err(|_| ApiError::internal())?;

        if sender_id == user_id {
            self.require(user_id, organization_id, "messages.delete_own").await?;
        } else {
            self.require(user_id, organization_id, "messages.delete_any").await?;
        }

        let res = sqlx::query(
            r#"
            update messages
            set deleted_at = $2
            where id = $1 and deleted_at is null
            "#,
        )
        .bind(message_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?;

        if res.rows_affected() != 1 {
            return Err(ApiError::not_found());
        }

        self.publish_event(
            organization_id,
            channel_id,
            user_id,
            "message.deleted",
            "message.deleted",
            json!({ "message_id": message_id }),
            now,
        )
        .await?;

        Ok(())
    }

    pub async fn add_reaction(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        emoji: String,
    ) -> Result<(), ApiError> {
        let (organization_id, channel_id) = self.message_org_channel(message_id).await?;
        self.require(user_id, organization_id, "messages.send").await?;

        sqlx::query(
            r#"
            insert into message_reactions (id, organization_id, message_id, user_id, emoji)
            values ($1, $2, $3, $4, $5)
            on conflict do nothing
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(organization_id)
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?;

        // optional: publish reaction event later
        let _ = channel_id;
        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        emoji: String,
    ) -> Result<(), ApiError> {
        let (organization_id, channel_id) = self.message_org_channel(message_id).await?;
        self.require(user_id, organization_id, "messages.send").await?;

        sqlx::query(
            r#"
            delete from message_reactions
            where message_id = $1 and user_id = $2 and emoji = $3
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?;

        let _ = channel_id;
        Ok(())
    }

    async fn require(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        permission: &'static str,
    ) -> Result<(), ApiError> {
        let membership = self
            .memberships
            .find_membership(organization_id, user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        if membership.role == "owner" {
            return Ok(());
        }

        let role = self
            .roles
            .find_by_org_and_name(organization_id, &membership.role)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::forbidden)?;

        let allowed = role
            .permissions
            .get(permission)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if allowed {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }

    async fn channel_org_and_boundary(
        &self,
        channel_id: Uuid,
        before: Option<Uuid>,
    ) -> Result<(Uuid, Option<(OffsetDateTime, Uuid)>), ApiError> {
        let ch = sqlx::query(
            r#"
            select id, organization_id
            from channels
            where id = $1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::not_found)?;

        let organization_id: Uuid = ch.try_get("organization_id").map_err(|_| ApiError::internal())?;

        let boundary = if let Some(before_id) = before {
            let row = sqlx::query(
                r#"
                select created_at, id
                from messages
                where id = $1 and channel_id = $2
                "#,
            )
            .bind(before_id)
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| ApiError::internal())?;

            row.map(|r| {
                (
                    r.try_get::<OffsetDateTime, _>("created_at").unwrap(),
                    r.try_get::<Uuid, _>("id").unwrap(),
                )
            })
        } else {
            None
        };

        Ok((organization_id, boundary))
    }

    async fn message_org_channel(&self, message_id: Uuid) -> Result<(Uuid, Uuid), ApiError> {
        let row = sqlx::query(
            r#"
            select organization_id, channel_id
            from messages
            where id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::not_found)?;

        Ok((
            row.try_get("organization_id").map_err(|_| ApiError::internal())?,
            row.try_get("channel_id").map_err(|_| ApiError::internal())?,
        ))
    }

    async fn nats(&self) -> Result<&async_nats::Client, ApiError> {
        let nats_url = self.nats_url.clone();
        self.nats
            .get_or_try_init(move || async move {
                async_nats::connect(nats_url)
                    .await
                    .map_err(|_| ApiError::internal())
            })
            .await
    }

    async fn publish_event(
        &self,
        organization_id: Uuid,
        channel_id: Uuid,
        actor_id: Uuid,
        subject_type: &str,
        event_type: &str,
        data: serde_json::Value,
        occurred_at: OffsetDateTime,
    ) -> Result<(), ApiError> {
        let subject = format!("org.{}.channel.{}.{}", organization_id, channel_id, subject_type);
        let evt = EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            r#type: event_type.to_string(),
            organization_id: organization_id.to_string(),
            channel_id: channel_id.to_string(),
            actor_id: actor_id.to_string(),
            occurred_at: occurred_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            data,
        };

        let payload = serde_json::to_vec(&evt).map_err(|_| ApiError::internal())?;
        self.nats()
            .await?
            .publish(subject, payload.into())
            .await
            .map_err(|_| ApiError::internal())?;
        Ok(())
    }
}
