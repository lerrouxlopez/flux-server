-- Notification profiles + rules (Work/Play), with user/channel overrides.

create table if not exists notification_profiles (
  id uuid primary key,
  organization_id uuid references organizations(id) on delete cascade,
  scope text not null default 'org', -- 'platform' | 'org'
  mode text not null,               -- 'work' | 'play'
  label text not null,
  description text,
  created_at timestamptz not null default now()
);

do $$
begin
  alter table notification_profiles
    add constraint notification_profiles_scope_check
    check (scope in ('platform','org'));
exception
  when duplicate_object then null;
end $$;

do $$
begin
  alter table notification_profiles
    add constraint notification_profiles_mode_check
    check (mode in ('work','play'));
exception
  when duplicate_object then null;
end $$;

create index if not exists notification_profiles_org_idx on notification_profiles (organization_id, mode);
create index if not exists notification_profiles_scope_idx on notification_profiles (scope, mode);

create table if not exists notification_profile_rules (
  profile_id uuid not null references notification_profiles(id) on delete cascade,
  rule text not null,
  enabled boolean not null default false,
  primary key (profile_id, rule)
);

-- User override (org-scoped, mode-aware)
create table if not exists user_notification_overrides (
  organization_id uuid not null references organizations(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  mode text not null,
  profile_id uuid references notification_profiles(id) on delete set null,
  updated_at timestamptz not null default now(),
  primary key (organization_id, user_id, mode)
);

do $$
begin
  alter table user_notification_overrides
    add constraint user_notification_overrides_mode_check
    check (mode in ('work','play'));
exception
  when duplicate_object then null;
end $$;

create index if not exists user_notification_overrides_user_idx
  on user_notification_overrides (user_id, organization_id);

-- Channel override (per user per channel)
create table if not exists channel_notification_overrides (
  channel_id uuid not null references channels(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  profile_id uuid references notification_profiles(id) on delete set null,
  updated_at timestamptz not null default now(),
  primary key (channel_id, user_id)
);

create index if not exists channel_notification_overrides_user_idx
  on channel_notification_overrides (user_id, channel_id);

-- Organization defaults: mode profiles first, then org fallback profile.
alter table organizations
  add column if not exists notification_work_profile_id uuid references notification_profiles(id),
  add column if not exists notification_play_profile_id uuid references notification_profiles(id),
  add column if not exists notification_default_profile_id uuid references notification_profiles(id);

-- Seed platform defaults (no "all messages" spam by default).
insert into notification_profiles (id, organization_id, scope, mode, label, description)
values
  (
    '11111111-1111-1111-1111-111111111111',
    null,
    'platform',
    'work',
    'Work (Default)',
    'Work mode defaults: mentions + thread replies + pin changes. No all-message spam.'
  ),
  (
    '22222222-2222-2222-2222-222222222222',
    null,
    'platform',
    'play',
    'Play (Default)',
    'Play mode defaults: mentions only. No all-message spam.'
  )
on conflict (id) do nothing;

-- Rules: each row is a boolean toggle for a given capability.
-- Known rules:
-- - message_all
-- - message_mentions
-- - thread_replies
-- - pin_changes
-- - media_events
insert into notification_profile_rules (profile_id, rule, enabled)
values
  -- Work default
  ('11111111-1111-1111-1111-111111111111', 'message_all', false),
  ('11111111-1111-1111-1111-111111111111', 'message_mentions', true),
  ('11111111-1111-1111-1111-111111111111', 'thread_replies', true),
  ('11111111-1111-1111-1111-111111111111', 'pin_changes', true),
  ('11111111-1111-1111-1111-111111111111', 'media_events', false),
  -- Play default
  ('22222222-2222-2222-2222-222222222222', 'message_all', false),
  ('22222222-2222-2222-2222-222222222222', 'message_mentions', true),
  ('22222222-2222-2222-2222-222222222222', 'thread_replies', false),
  ('22222222-2222-2222-2222-222222222222', 'pin_changes', false),
  ('22222222-2222-2222-2222-222222222222', 'media_events', false)
on conflict (profile_id, rule) do nothing;

