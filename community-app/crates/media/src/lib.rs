use anyhow::Context;
use domain::{MediaRoom, MediaRoomKind};
use livekit_api::access_token::{AccessToken, VideoGrants};
use permissions::{perms, Perms};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

pub const TOKEN_TTL_SECS: u64 = 60 * 60;

#[derive(Debug, Clone)]
pub struct LiveKitConfig {
    pub url: String,
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
        livekit_url: cfg.url.clone(),
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
        cfg.url.trim_end_matches('/')
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
