-- Enforce known channel kinds
alter table channels
  add constraint channels_kind_check
  check (kind in ('text', 'voice', 'announcement', 'private'));

create table messages (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid not null references channels(id) on delete cascade,
  sender_id uuid not null references users(id) on delete cascade,
  body text,
  kind text not null default 'text',
  created_at timestamptz not null default now(),
  edited_at timestamptz,
  deleted_at timestamptz,
  constraint messages_kind_check check (kind in ('text', 'system', 'attachment'))
);

create index messages_channel_created_idx
  on messages (channel_id, created_at desc);

create index messages_org_created_idx
  on messages (organization_id, created_at desc);

create table media_rooms (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid references channels(id) on delete set null,
  livekit_room_name text not null unique,
  kind text not null,
  created_by uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  constraint media_rooms_kind_check check (kind in ('voice', 'meeting', 'stage'))
);

create index media_rooms_org_idx on media_rooms (organization_id);
create index media_rooms_channel_idx on media_rooms (channel_id);

create table branding_profiles (
  organization_id uuid primary key references organizations(id) on delete cascade,
  app_name text not null,
  logo_url text,
  icon_url text,
  primary_color text,
  secondary_color text,
  custom_domain text unique,
  email_from_name text,
  privacy_url text,
  terms_url text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
