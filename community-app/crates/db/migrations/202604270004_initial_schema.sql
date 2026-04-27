-- Align auth table naming with the platform model
alter table user_sessions rename to refresh_tokens;
alter table refresh_tokens rename column refresh_token_hash to token_hash;

alter index if exists user_sessions_user_id_idx rename to refresh_tokens_user_id_idx;
alter index if exists user_sessions_refresh_token_hash_key rename to refresh_tokens_token_hash_idx;

-- Multi-tenant roles & permissions
create table roles (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  name text not null,
  permissions jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  unique (organization_id, name)
);

create index roles_org_idx on roles (organization_id);

-- Membership lookup index
create index org_members_user_idx
  on organization_members (user_id);

-- Message reactions (ephemeral UX state goes to Redis later; canonical reactions live here)
create table message_reactions (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  message_id uuid not null references messages(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  emoji text not null,
  created_at timestamptz not null default now(),
  unique (message_id, user_id, emoji)
);

create index message_reactions_message_idx on message_reactions (message_id);
create index message_reactions_org_idx on message_reactions (organization_id);

-- Attachments metadata (actual bytes go to object storage)
create table message_attachments (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  message_id uuid not null references messages(id) on delete cascade,
  uploader_id uuid not null references users(id) on delete cascade,
  url text not null,
  filename text,
  content_type text,
  size_bytes bigint,
  created_at timestamptz not null default now()
);

create index message_attachments_message_idx on message_attachments (message_id);
create index message_attachments_org_idx on message_attachments (organization_id);

-- Media sessions (join/leave tracking only; LiveKit moves media)
create table media_sessions (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  media_room_id uuid not null references media_rooms(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  joined_at timestamptz not null default now(),
  left_at timestamptz
);

create index media_sessions_room_joined_idx on media_sessions (media_room_id, joined_at desc);
create index media_sessions_org_idx on media_sessions (organization_id);

-- Branding: allow multiple nulls but enforce uniqueness for set domains
alter table branding_profiles
  drop constraint if exists branding_profiles_custom_domain_key;

create unique index branding_profiles_custom_domain_idx
  on branding_profiles (custom_domain)
  where custom_domain is not null;

-- Keyset pagination helper index (created_at + id)
create index messages_channel_created_id_idx
  on messages (channel_id, created_at desc, id desc);

-- Audit logs (append-only)
create table audit_logs (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  actor_id uuid references users(id) on delete set null,
  action text not null,
  target_type text,
  target_id uuid,
  data jsonb,
  occurred_at timestamptz not null default now()
);

create index audit_logs_org_occurred_idx
  on audit_logs (organization_id, occurred_at desc);
