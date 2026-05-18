use anyhow::Context;
use domain::{MediaParticipant, MediaRoom, MediaRoomKind, MediaSession};
use livekit_api::access_token::{AccessToken, VideoGrants};
use permissions::{perms, Perms};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub const TOKEN_TTL_SECS: u64 = 60 * 60;
pub const PARTICIPANT_STALE_AFTER_SECS: i64 = 90;

#[derive(Debug, Clone)]
pub struct LiveKitConfig {
    pub internal_url: String,
    pub public_url: String,
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateRoomKind {
    Voice,
    Meeting,
    Stage,
}

impl From<CreateRoomKind> for MediaRoomKind {
    fn from(value: CreateRoomKind) -> Self {
        match value {
            CreateRoomKind::Voice => MediaRoomKind::Voice,
            CreateRoomKind::Meeting => MediaRoomKind::Meeting,
            CreateRoomKind::Stage => MediaRoomKind::Stage,
        }
    }
}

pub fn stable_livekit_room_name(org_id: Uuid, room_id: Uuid) -> String {
    // Stable, globally unique, and not user-controlled.
    format!("org-{org_id}-room-{room_id}")
}

pub async fn create_media_room(
    pool: &PgPool,
    organization_id: Uuid,
    channel_id: Option<Uuid>,
    kind: MediaRoomKind,
    name: String,
    created_by: Uuid,
) -> anyhow::Result<MediaRoom> {
    let id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    let livekit_room_name = stable_livekit_room_name(organization_id, id);
    let kind_str = match kind {
        MediaRoomKind::Voice => "voice",
        MediaRoomKind::Meeting => "meeting",
        MediaRoomKind::Stage => "stage",
    };

    sqlx::query(
        r#"
        insert into media_rooms (id, organization_id, channel_id, livekit_room_name, kind, name, created_by, created_at)
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(id)
    .bind(organization_id)
    .bind(channel_id)
    .bind(&livekit_room_name)
    .bind(kind_str)
    .bind(&name)
    .bind(created_by)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(MediaRoom {
        id,
        organization_id,
        channel_id,
        livekit_room_name,
        kind,
        name,
        created_by,
        created_at: now,
    })
}

pub async fn get_media_room(pool: &PgPool, room_id: Uuid) -> anyhow::Result<Option<MediaRoom>> {
    let row = sqlx::query(
        r#"
        select id, organization_id, channel_id, livekit_room_name, kind, name, created_by, created_at
        from media_rooms
        where id = $1
        "#,
    )
    .bind(room_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let kind: String = row.get("kind");
    let kind = match kind.as_str() {
        "voice" => MediaRoomKind::Voice,
        "meeting" => MediaRoomKind::Meeting,
        "stage" => MediaRoomKind::Stage,
        _ => MediaRoomKind::Meeting,
    };

    Ok(Some(MediaRoom {
        id: row.get("id"),
        organization_id: row.get("organization_id"),
        channel_id: row.try_get::<Option<Uuid>, _>("channel_id").unwrap_or(None),
        livekit_room_name: row.get("livekit_room_name"),
        kind,
        name: row.get("name"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
    }))
}

// ---- Durable lifecycle: session + participant join/heartbeat/leave ----

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinIntent {
    Voice,
    Meeting,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JoinRequest {
    pub intent: JoinIntent,
    #[serde(default)]
    pub publish_audio: Option<bool>,
    #[serde(default)]
    pub publish_video: Option<bool>,
    #[serde(default)]
    pub publish_screen: Option<bool>,
    #[serde(default)]
    pub publish_data: Option<bool>,
    #[serde(default)]
    pub subscribe: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JoinResponse {
    pub session_id: Uuid,
    pub participant_id: Uuid,
    pub token: String,
    pub livekit_url: String,
    pub expires_at: OffsetDateTime,
    pub granted: GrantedCapabilities,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantedCapabilities {
    pub can_subscribe: bool,
    pub can_publish_audio: bool,
    pub can_publish_video: bool,
    pub can_publish_screen: bool,
    pub can_publish_data: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStatus {
    pub session: MediaSession,
    pub participants: Vec<MediaParticipant>,
}

pub async fn join_room(
    pool: &PgPool,
    livekit: &LiveKitConfig,
    room: &MediaRoom,
    user_id: Uuid,
    user_perms: Perms,
    req: JoinRequest,
) -> anyhow::Result<JoinResponse> {
    let can_join = permissions::has(user_perms, perms::VOICE_JOIN);
    if !can_join {
        anyhow::bail!("permission denied");
    }

    let defaults = defaults_for_intent(room.kind, req.intent);
    let requested = RequestedCapabilities {
        subscribe: req.subscribe.unwrap_or(defaults.subscribe),
        publish_audio: req.publish_audio.unwrap_or(defaults.publish_audio),
        publish_video: req.publish_video.unwrap_or(defaults.publish_video),
        publish_screen: req.publish_screen.unwrap_or(defaults.publish_screen),
        publish_data: req.publish_data.unwrap_or(defaults.publish_data),
    };

    // Ignore/downgrade: cap requested capabilities by permission bits.
    let granted = GrantedCapabilities {
        can_subscribe: requested.subscribe,
        can_publish_audio: requested.publish_audio
            && permissions::has(user_perms, perms::VOICE_SPEAK),
        can_publish_video: requested.publish_video
            && permissions::has(user_perms, perms::VIDEO_START),
        can_publish_screen: requested.publish_screen
            && permissions::has(user_perms, perms::SCREEN_SHARE),
        can_publish_data: requested.publish_data && permissions::has(user_perms, perms::VOICE_SPEAK),
    };

    let now = OffsetDateTime::now_utc();

    // Reuse an active session for the room, or create a new one.
    let session = get_or_create_active_session(pool, room.organization_id, room.id, user_id, now)
        .await?;

    let participant_id = Uuid::now_v7();
    let identity = user_id.to_string();

    sqlx::query(
        r#"
        insert into media_participants (
          id, organization_id, media_session_id, user_id, identity,
          can_subscribe, can_publish_audio, can_publish_video, can_publish_screen, can_publish_data,
          joined_at, last_heartbeat_at
        )
        values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        "#,
    )
    .bind(participant_id)
    .bind(room.organization_id)
    .bind(session.id)
    .bind(user_id)
    .bind(&identity)
    .bind(granted.can_subscribe)
    .bind(granted.can_publish_audio)
    .bind(granted.can_publish_video)
    .bind(granted.can_publish_screen)
    .bind(granted.can_publish_data)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let token_req = TokenRequest {
        can_publish: granted.can_publish_audio || granted.can_publish_video || granted.can_publish_screen,
        can_subscribe: granted.can_subscribe,
        can_publish_data: granted.can_publish_data,
    };
    let token = issue_livekit_token(
        livekit,
        identity,
        room.livekit_room_name.clone(),
        &token_req,
    )?;

    Ok(JoinResponse {
        session_id: session.id,
        participant_id,
        token: token.token,
        livekit_url: token.livekit_url,
        expires_at: now + Duration::seconds(TOKEN_TTL_SECS as i64),
        granted,
    })
}

pub async fn heartbeat(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<bool> {
    let now = OffsetDateTime::now_utc();
    let res = sqlx::query(
        r#"
        update media_participants
        set last_heartbeat_at = $3
        where media_session_id = $1
          and user_id = $2
          and left_at is null
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(res.rows_affected() > 0)
}

pub async fn leave(
    pool: &PgPool,
    livekit: &LiveKitConfig,
    session_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<bool> {
    let now = OffsetDateTime::now_utc();

    // Load room name for LiveKit kick (best-effort), and participant identity.
    let row = sqlx::query(
        r#"
        select p.identity, r.livekit_room_name
        from media_participants p
        join media_sessions s on s.id = p.media_session_id
        join media_rooms r on r.id = s.media_room_id
        where p.media_session_id = $1
          and p.user_id = $2
          and p.left_at is null
        order by p.joined_at desc
        limit 1
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(false);
    };

    let identity: String = row.get("identity");
    let room_name: String = row.get("livekit_room_name");

    let _ = remove_participant(livekit, &room_name, &identity).await;

    let res = sqlx::query(
        r#"
        update media_participants
        set left_at = $3, left_reason = 'leave'
        where media_session_id = $1
          and user_id = $2
          and left_at is null
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await?;

    // If the session has no remaining active participants, end it.
    end_session_if_empty(pool, session_id, now, Some("empty")).await?;

    Ok(res.rows_affected() > 0)
}

pub async fn get_session_status(
    pool: &PgPool,
    session_id: Uuid,
) -> anyhow::Result<Option<SessionStatus>> {
    let row = sqlx::query(
        r#"
        select id, organization_id, media_room_id, created_by, started_at, ended_at, ended_reason
        from media_sessions
        where id = $1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let session = MediaSession {
        id: row.get("id"),
        organization_id: row.get("organization_id"),
        media_room_id: row.get("media_room_id"),
        created_by: row.get("created_by"),
        started_at: row.get("started_at"),
        ended_at: row.try_get("ended_at").ok(),
        ended_reason: row.try_get::<Option<String>, _>("ended_reason").unwrap_or(None),
    };

    let rows = sqlx::query(
        r#"
        select
          id, organization_id, media_session_id, user_id, identity,
          can_subscribe, can_publish_audio, can_publish_video, can_publish_screen, can_publish_data,
          joined_at, last_heartbeat_at, left_at, left_reason, kick_attempted_at, kicked_at
        from media_participants
        where media_session_id = $1
        order by joined_at asc
        "#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    let participants = rows
        .into_iter()
        .map(|r| MediaParticipant {
            id: r.get("id"),
            organization_id: r.get("organization_id"),
            media_session_id: r.get("media_session_id"),
            user_id: r.get("user_id"),
            identity: r.get("identity"),
            can_subscribe: r.get("can_subscribe"),
            can_publish_audio: r.get("can_publish_audio"),
            can_publish_video: r.get("can_publish_video"),
            can_publish_screen: r.get("can_publish_screen"),
            can_publish_data: r.get("can_publish_data"),
            joined_at: r.get("joined_at"),
            last_heartbeat_at: r.get("last_heartbeat_at"),
            left_at: r.try_get("left_at").ok(),
            left_reason: r.try_get::<Option<String>, _>("left_reason").unwrap_or(None),
            kick_attempted_at: r.try_get("kick_attempted_at").ok(),
            kicked_at: r.try_get("kicked_at").ok(),
        })
        .collect();

    Ok(Some(SessionStatus { session, participants }))
}

pub async fn cleanup_stale_participants(
    pool: &PgPool,
    livekit: &LiveKitConfig,
    stale_after: Duration,
) -> anyhow::Result<i64> {
    let now = OffsetDateTime::now_utc();
    let cutoff = now - stale_after;

    let stale = sqlx::query(
        r#"
        select p.id, p.identity, p.media_session_id, r.livekit_room_name
        from media_participants p
        join media_sessions s on s.id = p.media_session_id
        join media_rooms r on r.id = s.media_room_id
        where p.left_at is null
          and p.last_heartbeat_at < $1
        order by p.last_heartbeat_at asc
        limit 250
        "#,
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let mut cleaned: i64 = 0;
    for row in stale {
        let participant_id: Uuid = row.get("id");
        let identity: String = row.get("identity");
        let session_id: Uuid = row.get("media_session_id");
        let room_name: String = row.get("livekit_room_name");

        let kicked = remove_participant(livekit, &room_name, &identity).await.is_ok();

        let _ = sqlx::query(
            r#"
            update media_participants
            set
              left_at = $2,
              left_reason = 'stale',
              kick_attempted_at = $2,
              kicked_at = case when $3 then $2 else kicked_at end
            where id = $1
              and left_at is null
            "#,
        )
        .bind(participant_id)
        .bind(now)
        .bind(kicked)
        .execute(pool)
        .await;

        let _ = end_session_if_empty(pool, session_id, now, Some("empty")).await;
        cleaned += 1;
    }

    Ok(cleaned)
}

#[derive(Debug, Clone, Copy)]
struct RequestedCapabilities {
    subscribe: bool,
    publish_audio: bool,
    publish_video: bool,
    publish_screen: bool,
    publish_data: bool,
}

fn defaults_for_intent(room_kind: MediaRoomKind, intent: JoinIntent) -> RequestedCapabilities {
    match (room_kind, intent) {
        (MediaRoomKind::Voice, JoinIntent::Voice) => RequestedCapabilities {
            subscribe: true,
            publish_audio: true,
            publish_video: false,
            publish_screen: false,
            publish_data: true,
        },
        (_, JoinIntent::Meeting) => RequestedCapabilities {
            subscribe: true,
            publish_audio: true,
            publish_video: true,
            publish_screen: true,
            publish_data: true,
        },
        _ => RequestedCapabilities {
            subscribe: true,
            publish_audio: true,
            publish_video: false,
            publish_screen: false,
            publish_data: true,
        },
    }
}

async fn get_or_create_active_session(
    pool: &PgPool,
    organization_id: Uuid,
    media_room_id: Uuid,
    created_by: Uuid,
    now: OffsetDateTime,
) -> anyhow::Result<MediaSession> {
    if let Some(row) = sqlx::query(
        r#"
        select id, organization_id, media_room_id, created_by, started_at, ended_at, ended_reason
        from media_sessions
        where media_room_id = $1
          and ended_at is null
        order by started_at desc
        limit 1
        "#,
    )
    .bind(media_room_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(MediaSession {
            id: row.get("id"),
            organization_id: row.get("organization_id"),
            media_room_id: row.get("media_room_id"),
            created_by: row.get("created_by"),
            started_at: row.get("started_at"),
            ended_at: row.try_get("ended_at").ok(),
            ended_reason: row.try_get::<Option<String>, _>("ended_reason").unwrap_or(None),
        });
    }

    let id = Uuid::now_v7();
    sqlx::query(
        r#"
        insert into media_sessions (id, organization_id, media_room_id, created_by, started_at)
        values ($1,$2,$3,$4,$5)
        "#,
    )
    .bind(id)
    .bind(organization_id)
    .bind(media_room_id)
    .bind(created_by)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(MediaSession {
        id,
        organization_id,
        media_room_id,
        created_by,
        started_at: now,
        ended_at: None,
        ended_reason: None,
    })
}

async fn end_session_if_empty(
    pool: &PgPool,
    session_id: Uuid,
    now: OffsetDateTime,
    ended_reason: Option<&str>,
) -> anyhow::Result<()> {
    let active_count: i64 = sqlx::query_scalar(
        r#"
        select count(1)::bigint
        from media_participants
        where media_session_id = $1
          and left_at is null
        "#,
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    if active_count > 0 {
        return Ok(());
    }

    let _ = sqlx::query(
        r#"
        update media_sessions
        set ended_at = coalesce(ended_at, $2),
            ended_reason = coalesce(ended_reason, $3)
        where id = $1
        "#,
    )
    .bind(session_id)
    .bind(now)
    .bind(ended_reason.map(|s| s.to_string()))
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub can_publish: bool,
    pub can_subscribe: bool,
    pub can_publish_data: bool,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub livekit_url: String,
}

pub fn required_perms_for_token(room_kind: MediaRoomKind, req: &TokenRequest) -> Perms {
    let mut needed = perms::VOICE_JOIN;

    if req.can_publish {
        needed |= perms::VOICE_SPEAK;

        // Be conservative: meeting/stage typically implies video + screen share.
        if matches!(room_kind, MediaRoomKind::Meeting | MediaRoomKind::Stage) {
            needed |= perms::VIDEO_START | perms::SCREEN_SHARE;
        }
    }

    needed
}

pub fn issue_livekit_token(
    cfg: &LiveKitConfig,
    identity: String,
    room_name: String,
    req: &TokenRequest,
) -> anyhow::Result<TokenResponse> {
    let _span = tracing::info_span!("livekit.token.generate", room_name=%room_name).entered();
    let grants = VideoGrants {
        room_join: true,
        room: room_name,
        can_publish: req.can_publish,
        can_subscribe: req.can_subscribe,
        can_publish_data: req.can_publish_data,
        ..Default::default()
    };

    let mut token = AccessToken::with_api_key(&cfg.api_key, &cfg.api_secret)
        .with_identity(&identity)
        .with_grants(grants);
    token = token.with_ttl(std::time::Duration::from_secs(TOKEN_TTL_SECS));
    let jwt = token.to_jwt().context("failed to create livekit jwt")?;

    Ok(TokenResponse {
        token: jwt,
        livekit_url: cfg.public_url.clone(),
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct ListParticipantsRequestBody {
    room: String,
}

pub async fn list_participants(
    cfg: &LiveKitConfig,
    room_name: &str,
) -> anyhow::Result<serde_json::Value> {
    // Call LiveKit RoomService API via Twirp; authorize with a server-generated token.
    let grants = VideoGrants {
        room_admin: true,
        room: room_name.to_string(),
        ..Default::default()
    };

    let mut token = AccessToken::with_api_key(&cfg.api_key, &cfg.api_secret)
        .with_identity("server")
        .with_grants(grants);
    token = token.with_ttl(std::time::Duration::from_secs(60));
    let jwt = token.to_jwt().context("failed to create livekit jwt")?;

    let url = format!(
        "{}/twirp/livekit.RoomService/ListParticipants",
        cfg.internal_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .bearer_auth(jwt)
        .json(&ListParticipantsRequestBody {
            room: room_name.to_string(),
        })
        .send()
        .await?
        .error_for_status()?;

    Ok(res.json::<serde_json::Value>().await?)
}

#[derive(Debug, Deserialize, Serialize)]
struct RemoveParticipantRequestBody {
    room: String,
    identity: String,
}

pub async fn remove_participant(
    cfg: &LiveKitConfig,
    room_name: &str,
    identity: &str,
) -> anyhow::Result<()> {
    let grants = VideoGrants {
        room_admin: true,
        room: room_name.to_string(),
        ..Default::default()
    };

    let mut token = AccessToken::with_api_key(&cfg.api_key, &cfg.api_secret)
        .with_identity("server")
        .with_grants(grants);
    token = token.with_ttl(std::time::Duration::from_secs(60));
    let jwt = token.to_jwt().context("failed to create livekit jwt")?;

    let url = format!(
        "{}/twirp/livekit.RoomService/RemoveParticipant",
        cfg.internal_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let _ = client
        .post(url)
        .bearer_auth(jwt)
        .json(&RemoveParticipantRequestBody {
            room: room_name.to_string(),
            identity: identity.to_string(),
        })
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
