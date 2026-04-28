-- Core entities (multi-tenant, centralized; no federation).

-- Organizations (already created in init)
alter table organizations
  add column if not exists updated_at timestamptz not null default now();

-- Users (already created in init)
alter table users
  add column if not exists updated_at timestamptz not null default now();

-- Organization members (already created in init)

-- Channels: constrain kind to known values
alter table channels
  add constraint if not exists channels_kind_check
  check (kind in ('text', 'voice', 'announcement', 'private'));

-- Messages
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

create index if not exists messages_org_channel_created_idx
  on messages (organization_id, channel_id, created_at desc);

-- Media rooms (LiveKit room mapping; no media transport in Rust)
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

create unique index if not exists media_rooms_org_livekit_room_uq
  on media_rooms (organization_id, livekit_room_name);

create index if not exists media_rooms_org_created_idx
  on media_rooms (organization_id, created_at desc);

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

