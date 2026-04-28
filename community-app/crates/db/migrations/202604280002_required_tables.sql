-- Required tables for the initial schema.
-- Postgres is source of truth; Redis is ephemeral; NATS events published after commit.

-- Roles (org-scoped)
create table if not exists roles (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  name text not null,
  permissions bigint not null default 0,
  created_at timestamptz not null default now(),
  unique (organization_id, name)
);

create index if not exists roles_org_idx on roles (organization_id);

-- Constrain channel kinds
alter table channels
  add constraint if not exists channels_kind_check
  check (kind in ('text', 'voice', 'announcement', 'private'));

-- Messages (soft-delete via deleted_at)
create table if not exists messages (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid not null references channels(id) on delete cascade,
  sender_id uuid not null references users(id) on delete cascade,
  body text,
  kind text not null,
  created_at timestamptz not null default now(),
  edited_at timestamptz,
  deleted_at timestamptz
);

alter table messages
  add constraint if not exists messages_kind_check
  check (kind in ('text', 'system', 'attachment'));

-- Required indexes (plus an extra composite for keyset pagination)
create index if not exists messages_channel_created_idx
  on messages (channel_id, created_at desc);

create index if not exists messages_org_created_idx
  on messages (organization_id, created_at desc);

create index if not exists messages_channel_created_id_idx
  on messages (channel_id, created_at desc, id desc);

-- Message reactions (org-scoped)
create table if not exists message_reactions (
  organization_id uuid not null references organizations(id) on delete cascade,
  message_id uuid not null references messages(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  emoji text not null,
  created_at timestamptz not null default now(),
  primary key (message_id, user_id, emoji)
);

create index if not exists message_reactions_org_idx on message_reactions (organization_id, created_at desc);

-- Message attachments
create table if not exists message_attachments (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  message_id uuid not null references messages(id) on delete cascade,
  uploader_id uuid not null references users(id) on delete cascade,
  filename text not null,
  content_type text,
  size_bytes bigint not null,
  storage_path text not null,
  created_at timestamptz not null default now()
);

create index if not exists message_attachments_message_idx on message_attachments (message_id);
create index if not exists message_attachments_org_idx on message_attachments (organization_id, created_at desc);

-- Media rooms (LiveKit room mapping)
create table if not exists media_rooms (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid references channels(id) on delete set null,
  livekit_room_name text not null,
  kind text not null,
  created_by uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now()
);

alter table media_rooms
  add constraint if not exists media_rooms_kind_check
  check (kind in ('voice', 'meeting', 'stage'));

create index if not exists media_rooms_org_idx
  on media_rooms (organization_id);

create unique index if not exists media_rooms_org_livekit_room_uq
  on media_rooms (organization_id, livekit_room_name);

-- Media sessions (user participation/records; no raw A/V processed by Rust)
create table if not exists media_sessions (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  media_room_id uuid not null references media_rooms(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  livekit_participant_id text,
  started_at timestamptz not null default now(),
  ended_at timestamptz
);

create index if not exists media_sessions_org_idx on media_sessions (organization_id, started_at desc);
create index if not exists media_sessions_room_idx on media_sessions (media_room_id, started_at desc);

-- Branding profile (1:1 per organization)
create table if not exists branding_profiles (
  organization_id uuid primary key references organizations(id) on delete cascade,
  app_name text not null,
  logo_url text,
  icon_url text,
  primary_color text,
  secondary_color text,
  custom_domain text,
  email_from_name text,
  privacy_url text,
  terms_url text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists branding_profiles_custom_domain_idx
  on branding_profiles (custom_domain)
  where custom_domain is not null;

-- Refresh tokens (DB-backed, hashed; JWT access tokens are stateless)
create table if not exists refresh_tokens (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  token_hash text not null,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  revoked_at timestamptz
);

create index if not exists refresh_tokens_user_idx on refresh_tokens (user_id, created_at desc);
create index if not exists refresh_tokens_expires_idx on refresh_tokens (expires_at);

-- Audit logs (org-scoped)
create table if not exists audit_logs (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  actor_user_id uuid references users(id) on delete set null,
  action text not null,
  target_type text,
  target_id uuid,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists audit_logs_org_created_idx on audit_logs (organization_id, created_at desc);

