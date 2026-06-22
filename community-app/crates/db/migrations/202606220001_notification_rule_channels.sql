-- Expand notification rules from a single `enabled` flag to per-channel
-- (in_app / desktop / sound) booleans, add 4 new rule kinds, let a profile
-- be owned by a user (personal custom profile, not just platform/org-shared),
-- and add quiet hours to the per-user override.

alter table notification_profile_rules rename column enabled to in_app;
alter table notification_profile_rules add column if not exists desktop boolean not null default false;
alter table notification_profile_rules add column if not exists sound boolean not null default false;

-- Backfill desktop/sound for the 5 pre-existing rules to match the
-- intent the frontend mock already had (apps/shell-web NotificationsPage
-- DEFAULT_RULES) so this migration is a translation, not a behavior change.
update notification_profile_rules set desktop = true
  where rule = 'thread_replies' and profile_id = '11111111-1111-1111-1111-111111111111';
update notification_profile_rules set desktop = true, sound = true
  where rule = 'message_mentions';

-- New rule kinds: mention_channel, friend_request, reaction, direct_message.
insert into notification_profile_rules (profile_id, rule, in_app, desktop, sound)
values
  -- Work default
  ('11111111-1111-1111-1111-111111111111', 'mention_channel', true,  true,  false),
  ('11111111-1111-1111-1111-111111111111', 'friend_request',  true,  true,  true),
  ('11111111-1111-1111-1111-111111111111', 'reaction',        false, false, false),
  ('11111111-1111-1111-1111-111111111111', 'direct_message',  true,  true,  true),
  -- Play default
  ('22222222-2222-2222-2222-222222222222', 'mention_channel', true,  true,  false),
  ('22222222-2222-2222-2222-222222222222', 'friend_request',  true,  true,  true),
  ('22222222-2222-2222-2222-222222222222', 'reaction',        false, false, false),
  ('22222222-2222-2222-2222-222222222222', 'direct_message',  true,  true,  true)
on conflict (profile_id, rule) do nothing;

-- A profile can be a personal custom profile owned by the member who
-- created it (created_by set, scope='org'), not just a platform/org-shared
-- preset (created_by null).
alter table notification_profiles add column if not exists created_by uuid references users(id) on delete set null;

-- Quiet hours, per user per org per mode (same row as the profile override).
alter table user_notification_overrides add column if not exists quiet_hours_enabled boolean not null default false;
alter table user_notification_overrides add column if not exists quiet_from time;
alter table user_notification_overrides add column if not exists quiet_to time;
alter table user_notification_overrides add column if not exists quiet_priority_override boolean not null default true;
