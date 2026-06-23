-- Lorelei's dedicated per-org channel uses a new channel kind.
alter table channels drop constraint if exists channels_kind_check;
alter table channels
  add constraint channels_kind_check
  check (kind in ('text', 'voice', 'video', 'announcement', 'private', 'dm', 'lorelei'));

-- Synthetic per-org bot user that posts Lorelei's replies. Excluded from member counts,
-- "online members" lists, and invite flows at the application layer.
alter table users add column if not exists is_system_bot boolean not null default false;
