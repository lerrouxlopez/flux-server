use crate::protocol::{ClientEvent, ServerEvent};
use axum::extract::ws::{Message, WebSocket};
use dashmap::{DashMap, DashSet};
use futures_util::{sink::SinkExt, stream::StreamExt};
use redis::AsyncCommands;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

const PRESENCE_TTL_SECS: u64 = 90;
const PRESENCE_REFRESH_SECS: u64 = 30;
const TYPING_TTL_SECS: u64 = 5;

#[derive(Clone)]
pub struct Runtime {
    hub: Arc<Hub>,
    nats: async_nats::Client,
    pool: PgPool,
}

pub struct SocketContext {
    pub user_id: Uuid,
    pub org_ids: Vec<Uuid>,
    pub redis: redis::aio::ConnectionManager,
    pub nats: async_nats::Client,
    pub pool: PgPool,
}

impl Runtime {
    pub fn new(nats: async_nats::Client, pool: PgPool) -> Self {
        Self {
            hub: Arc::new(Hub::new()),
            nats,
            pool,
        }
    }

    pub fn spawn_nats_fanout(&self) {
        let hub = self.hub.clone();
        let nats = self.nats.clone();
        let pool = self.pool.clone();

        tokio::spawn(async move {
            let mut sub_msg = match nats.subscribe("org.*.channel.*.message.created").await {
                Ok(s) => s,
                Err(e) => {
                    warn!(?e, "nats subscribe failed (message.created)");
                    return;
                }
            };
            let mut sub_presence = match nats.subscribe("org.*.presence.changed").await {
                Ok(s) => s,
                Err(e) => {
                    warn!(?e, "nats subscribe failed (presence.changed)");
                    return;
                }
            };
            let mut sub_typing = match nats.subscribe("channel.*.typing.started").await {
                Ok(s) => s,
                Err(e) => {
                    warn!(?e, "nats subscribe failed (typing.started)");
                    return;
                }
            };

            loop {
                tokio::select! {
                    msg = sub_msg.next() => {
                        let Some(msg) = msg else { break; };
                        if let Err(e) = handle_message_created(&hub, &pool, msg.payload).await {
                            debug!(?e, "failed to handle message.created");
                        }
                    }
                    msg = sub_presence.next() => {
                        let Some(msg) = msg else { break; };
                        if let Ok(evt) = serde_json::from_slice::<ServerEvent>(&msg.payload) {
                            hub.broadcast_event(&evt);
                        }
                    }
                    msg = sub_typing.next() => {
                        let Some(msg) = msg else { break; };
                        if let Ok(evt) = serde_json::from_slice::<ServerEvent>(&msg.payload) {
                            hub.broadcast_event(&evt);
                        }
                    }
                }
            }
        });
    }

    pub async fn handle_socket(
        &self,
        ctx: SocketContext,
        socket: WebSocket,
    ) -> anyhow::Result<()> {
        self.hub
            .handle_socket(ctx.user_id, ctx.org_ids, ctx.redis, ctx.nats, ctx.pool, socket)
            .await
    }
}

struct ConnHandle {
    org_ids: Vec<Uuid>,
    tx: mpsc::UnboundedSender<Message>,
}

struct Hub {
    conns: DashMap<Uuid, ConnHandle>,
    org_index: DashMap<Uuid, DashSet<Uuid>>,     // org_id -> conn_ids
    channel_index: DashMap<Uuid, DashSet<Uuid>>, // channel_id -> conn_ids
}

impl Hub {
    fn new() -> Self {
        Self {
            conns: DashMap::new(),
            org_index: DashMap::new(),
            channel_index: DashMap::new(),
        }
    }

    fn insert_conn(&self, conn_id: Uuid, handle: ConnHandle) {
        for org_id in handle.org_ids.iter().copied() {
            self.org_index
                .entry(org_id)
                .or_insert_with(DashSet::new)
                .insert(conn_id);
        }
        self.conns.insert(conn_id, handle);
    }

    fn remove_conn(&self, conn_id: Uuid) {
        if let Some((_, handle)) = self.conns.remove(&conn_id) {
            for org_id in handle.org_ids {
                if let Some(set) = self.org_index.get(&org_id) {
                    set.remove(&conn_id);
                }
            }
        }
        for item in self.channel_index.iter() {
            item.value().remove(&conn_id);
        }
    }

    fn broadcast_to_org(&self, org_id: Uuid, evt: &ServerEvent) {
        let Some(set) = self.org_index.get(&org_id) else { return; };
        for conn_id in set.iter() {
            if let Some(conn) = self.conns.get(conn_id.key()) {
                let _ = conn.tx.send(Message::Text(serde_json::to_string(evt).unwrap_or_default().into()));
            }
        }
    }

    fn broadcast_to_channel(&self, channel_id: Uuid, evt: &ServerEvent) {
        let Some(set) = self.channel_index.get(&channel_id) else { return; };
        for conn_id in set.iter() {
            if let Some(conn) = self.conns.get(conn_id.key()) {
                let _ = conn.tx.send(Message::Text(serde_json::to_string(evt).unwrap_or_default().into()));
            }
        }
    }

    fn broadcast_event(&self, evt: &ServerEvent) {
        match evt {
            ServerEvent::MessageCreated {
                organization_id,
                channel_id,
                ..
            } => {
                // Prefer channel subscribers, but always fall back to org fanout.
                self.broadcast_to_channel(*channel_id, evt);
                self.broadcast_to_org(*organization_id, evt);
            }
            ServerEvent::PresenceChanged { organization_id, .. } => {
                self.broadcast_to_org(*organization_id, evt);
            }
            ServerEvent::TypingStarted { channel_id, .. } => {
                self.broadcast_to_channel(*channel_id, evt);
            }
        }
    }

    async fn handle_socket(
        &self,
        user_id: Uuid,
        org_ids: Vec<Uuid>,
        mut redis: redis::aio::ConnectionManager,
        nats: async_nats::Client,
        pool: PgPool,
        socket: WebSocket,
    ) -> anyhow::Result<()> {
        let conn_id = Uuid::now_v7();
        info!(%user_id, %conn_id, "ws connected");

        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        self.insert_conn(
            conn_id,
            ConnHandle {
                org_ids: org_ids.clone(),
                tx,
            },
        );

        // Presence: mark online per org and keep refreshing while socket is alive.
        let presence_task = tokio::spawn({
            let org_ids = org_ids.clone();
            let mut redis = redis.clone();
            let nats = nats.clone();
            async move {
                if let Err(e) = presence_set_online(&mut redis, &nats, user_id, &org_ids).await {
                    debug!(?e, "presence online failed");
                }
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(PRESENCE_REFRESH_SECS));
                loop {
                    tick.tick().await;
                    if let Err(e) = presence_refresh(&mut redis, user_id, &org_ids).await {
                        debug!(?e, "presence refresh failed");
                    }
                }
            }
        });

        let (mut ws_sender, mut ws_receiver) = socket.split();

        // Writer
        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Reader
        while let Some(inbound) = ws_receiver.next().await {
            let Ok(inbound) = inbound else { break };
            match inbound {
                Message::Text(txt) => {
                    if let Ok(ev) = serde_json::from_str::<ClientEvent>(&txt) {
                        if let Err(e) = handle_client_event(self, conn_id, &pool, &mut redis, &nats, user_id, &org_ids, ev).await {
                            debug!(?e, "client event handling failed");
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        self.remove_conn(conn_id);
        presence_task.abort();
        write_task.abort();

        // Best-effort: mark offline.
        if let Err(e) = presence_set_offline(&mut redis, &nats, user_id, &org_ids).await {
            debug!(?e, "presence offline failed");
        }
        info!(%user_id, %conn_id, "ws disconnected");
        Ok(())
    }
}

async fn handle_client_event(
    hub: &Hub,
    conn_id: Uuid,
    pool: &PgPool,
    redis: &mut redis::aio::ConnectionManager,
    nats: &async_nats::Client,
    user_id: Uuid,
    org_ids: &[Uuid],
    ev: ClientEvent,
) -> anyhow::Result<()> {
    match ev {
        ClientEvent::Ping => Ok(()),
        ClientEvent::TypingStart { channel_id } => {
            let org_id = ensure_channel_access(pool, user_id, org_ids, channel_id).await?;
            hub.channel_index
                .entry(channel_id)
                .or_insert_with(DashSet::new)
                .insert(conn_id);

            let typing_key = format!("typing:{channel_id}:{user_id}");
            let _: () = redis
                .set_ex(typing_key, "1", TYPING_TTL_SECS)
                .await
                .unwrap_or(());

            let evt = ServerEvent::TypingStarted { channel_id, user_id };
            let payload = serde_json::to_vec(&evt)?;
            let subject = format!("channel.{channel_id}.typing.started");
            let _ = nats.publish(subject, payload.into()).await;

            // Local broadcast for this node.
            hub.broadcast_to_channel(channel_id, &evt);
            // Best available "channel members" fanout (we don't have per-channel membership tables yet):
            // broadcast to all org member connections too.
            hub.broadcast_to_org(org_id, &evt);
            Ok(())
        }
        ClientEvent::TypingStop { channel_id } => {
            // Currently just stops local channel subscription; no server event required by spec.
            let _ = ensure_channel_access(pool, user_id, org_ids, channel_id).await?;
            // No-op if not present.
            if let Some(set) = hub.channel_index.get(&channel_id) {
                set.remove(&conn_id);
            }
            let typing_key = format!("typing:{channel_id}:{user_id}");
            let _: () = redis.del(typing_key).await.unwrap_or(());
            Ok(())
        }
    }
}

async fn ensure_channel_access(
    pool: &PgPool,
    user_id: Uuid,
    org_ids: &[Uuid],
    channel_id: Uuid,
) -> anyhow::Result<Uuid> {
    let row = sqlx::query_scalar::<_, Uuid>(
        r#"
        select c.organization_id
        from channels c
        join organization_members m
          on m.organization_id = c.organization_id
         and m.user_id = $2
        where c.id = $1
        "#,
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let Some(org_id) = row else {
        anyhow::bail!("forbidden");
    };
    if !org_ids.contains(&org_id) {
        anyhow::bail!("forbidden");
    }
    Ok(org_id)
}

async fn presence_set_online(
    redis: &mut redis::aio::ConnectionManager,
    nats: &async_nats::Client,
    user_id: Uuid,
    org_ids: &[Uuid],
) -> anyhow::Result<()> {
    // Track websocket connection count so multiple tabs/nodes don't flap presence.
    let ws_key = format!("ws:user:{user_id}");
    let count: i64 = redis.incr(&ws_key, 1).await.unwrap_or(1);
    let _: () = redis
        .expire(&ws_key, PRESENCE_TTL_SECS as i64)
        .await
        .unwrap_or(());

    let presence_key = format!("presence:user:{user_id}");
    let was_offline = count == 1;
    let _: () = redis
        .set_ex(&presence_key, "online", PRESENCE_TTL_SECS)
        .await
        .unwrap_or(());

    if was_offline {
        for org_id in org_ids.iter().copied() {
            let evt = ServerEvent::PresenceChanged {
                organization_id: org_id,
                user_id,
                status: "online".to_string(),
            };
            let payload = serde_json::to_vec(&evt)?;
            let subject = format!("org.{org_id}.presence.changed");
            let _ = nats.publish(subject, payload.into()).await;
        }
    }
    Ok(())
}

async fn presence_refresh(
    redis: &mut redis::aio::ConnectionManager,
    user_id: Uuid,
    org_ids: &[Uuid],
) -> anyhow::Result<()> {
    let ws_key = format!("ws:user:{user_id}");
    let presence_key = format!("presence:user:{user_id}");

    let _: () = redis
        .expire(&ws_key, PRESENCE_TTL_SECS as i64)
        .await
        .unwrap_or(());
    let _: () = redis
        .expire(&presence_key, PRESENCE_TTL_SECS as i64)
        .await
        .unwrap_or(());

    // org_ids is unused for refresh right now; keep signature stable.
    let _ = org_ids;
    Ok(())
}

async fn presence_set_offline(
    redis: &mut redis::aio::ConnectionManager,
    nats: &async_nats::Client,
    user_id: Uuid,
    org_ids: &[Uuid],
) -> anyhow::Result<()> {
    let ws_key = format!("ws:user:{user_id}");
    let presence_key = format!("presence:user:{user_id}");

    let count: i64 = redis.decr(&ws_key, 1).await.unwrap_or(0);
    if count <= 0 {
        let _: () = redis.del(&ws_key).await.unwrap_or(());
        let _: () = redis.del(&presence_key).await.unwrap_or(());

        for org_id in org_ids.iter().copied() {
            let evt = ServerEvent::PresenceChanged {
                organization_id: org_id,
                user_id,
                status: "offline".to_string(),
            };
            let payload = serde_json::to_vec(&evt)?;
            let subject = format!("org.{org_id}.presence.changed");
            let _ = nats.publish(subject, payload.into()).await;
        }
    } else {
        let _: () = redis
            .expire(&ws_key, PRESENCE_TTL_SECS as i64)
            .await
            .unwrap_or(());
        let _: () = redis
            .expire(&presence_key, PRESENCE_TTL_SECS as i64)
            .await
            .unwrap_or(());
    }
    Ok(())
}

async fn handle_message_created(hub: &Hub, pool: &PgPool, payload: bytes::Bytes) -> anyhow::Result<()> {
    // New format: typed envelope from `events` crate.
    if let Ok(env) =
        serde_json::from_slice::<events::envelope::EventEnvelope<MessageCreatedData>>(&payload)
    {
        let message = if let Some(m) = env.data.message {
            m
        } else if let Some(message_id) = env.data.message_id {
            fetch_message(pool, message_id).await?.unwrap_or(Value::Null)
        } else {
            Value::Null
        };

        let evt = ServerEvent::MessageCreated {
            organization_id: env.organization_id,
            channel_id: env.data.channel_id,
            message,
        };
        hub.broadcast_event(&evt);
        return Ok(());
    }

    // Back-compat: older gateway payloads.
    let evt_in: LegacyMessageCreated = serde_json::from_slice(&payload)?;

    let message = if let Some(m) = evt_in.message {
        m
    } else if let Some(message_id) = evt_in.message_id {
        fetch_message(pool, message_id).await?.unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    let evt = ServerEvent::MessageCreated {
        organization_id: evt_in.organization_id,
        channel_id: evt_in.channel_id,
        message,
    };
    hub.broadcast_event(&evt);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct MessageCreatedData {
    channel_id: Uuid,
    #[serde(default)]
    message: Option<Value>,
    #[serde(default)]
    message_id: Option<Uuid>,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyMessageCreated {
    organization_id: Uuid,
    channel_id: Uuid,
    #[serde(default)]
    message: Option<Value>,
    #[serde(default)]
    message_id: Option<Uuid>,
}

async fn fetch_message(pool: &PgPool, message_id: Uuid) -> anyhow::Result<Option<Value>> {
    let row = sqlx::query(
        r#"
        select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
        from messages
        where id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None); };

    let v = serde_json::json!({
        "id": row.get::<Uuid,_>("id"),
        "organization_id": row.get::<Uuid,_>("organization_id"),
        "channel_id": row.get::<Uuid,_>("channel_id"),
        "sender_id": row.get::<Uuid,_>("sender_id"),
        "body": row.try_get::<Option<String>,_>("body").ok().flatten(),
        "kind": row.get::<String,_>("kind"),
        "created_at": row.get::<time::OffsetDateTime,_>("created_at"),
        "edited_at": row.try_get::<Option<time::OffsetDateTime>,_>("edited_at").ok().flatten(),
        "deleted_at": row.try_get::<Option<time::OffsetDateTime>,_>("deleted_at").ok().flatten(),
    });
    Ok(Some(v))
}
