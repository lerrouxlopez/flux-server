-- Durable media session lifecycle: sessions + participants (heartbeat/leave/stale cleanup).
-- This replaces the earlier v1 `media_sessions` shape (which was unused by code).

-- If an older `media_sessions` table exists, drop it and recreate in the v2 shape.
drop table if exists media_sessions;

create table if not exists media_sessions (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  media_room_id uuid not null references media_rooms(id) on delete cascade,
  created_by uuid not null references users(id) on delete cascade,
  started_at timestamptz not null default now(),
  ended_at timestamptz,
  ended_reason text
);

create index if not exists media_sessions_org_idx
  on media_sessions (organization_id, started_at desc);

create index if not exists media_sessions_room_idx
  on media_sessions (media_room_id, started_at desc);

create index if not exists media_sessions_active_room_idx
  on media_sessions (media_room_id, started_at desc)
  where ended_at is null;

create table if not exists media_participants (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  media_session_id uuid not null references media_sessions(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  -- LiveKit identity we use for the participant (currently `user_id.to_string()`).
  identity text not null,
  -- Capabilities granted (server-derived).
  can_subscribe boolean not null default true,
  can_publish_audio boolean not null default false,
  can_publish_video boolean not null default false,
  can_publish_screen boolean not null default false,
  can_publish_data boolean not null default false,
  joined_at timestamptz not null default now(),
  last_heartbeat_at timestamptz not null default now(),
  left_at timestamptz,
  left_reason text,
  kick_attempted_at timestamptz,
  kicked_at timestamptz
);

create index if not exists media_participants_session_idx
  on media_participants (media_session_id, joined_at desc);

create index if not exists media_participants_active_idx
  on media_participants (media_session_id, last_heartbeat_at desc)
  where left_at is null;

create index if not exists media_participants_org_user_active_idx
  on media_participants (organization_id, user_id, joined_at desc)
  where left_at is null;

