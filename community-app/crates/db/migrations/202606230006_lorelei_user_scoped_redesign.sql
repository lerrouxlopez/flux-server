-- Lorelei redesign: she's a single global user now (previous migration), so the org-level
-- "settings" (display name, enable toggle, default provider/model, dedicated channel) no
-- longer apply — those were properties of a per-org bot user that no longer exists.
--
-- What's left at the org level is just the org-channel-scope Lorelei tenant/agent pair,
-- provisioned lazily the first time an admin adds her to a channel (see
-- crates/lorelei-bridge::provision_org_lorelei and routes_lorelei.rs::add_channel).
alter table org_lorelei_settings drop constraint if exists org_lorelei_settings_provider_check;
alter table org_lorelei_settings drop column if exists enabled;
alter table org_lorelei_settings drop column if exists display_name;
alter table org_lorelei_settings drop column if exists default_provider;
alter table org_lorelei_settings drop column if exists default_model;
alter table org_lorelei_settings drop column if exists bot_user_id;
alter table org_lorelei_settings drop column if exists default_channel_id;
alter table org_lorelei_settings alter column lorelei_tenant_id set not null;
alter table org_lorelei_settings alter column lorelei_agent_id set not null;

-- Personal PM scope: one Lorelei tenant/agent + DM channel per (user, org) pair, created
-- lazily on a user's first PM to her in that org (crates/lorelei-bridge::load_or_create_user_thread).
-- Scoped per-org rather than globally per-user so a side-project org's conversations don't
-- bleed into a work org's conversations with the same person.
create table if not exists user_lorelei_threads (
  user_id uuid not null references users(id) on delete cascade,
  organization_id uuid not null references organizations(id) on delete cascade,
  lorelei_tenant_id uuid not null,
  lorelei_agent_id uuid not null,
  dm_channel_id uuid not null references channels(id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (user_id, organization_id)
);

-- The dedicated-per-org-channel concept (kind = 'lorelei') is gone — PMs (kind = 'dm') cover
-- that role now. Revert the kind list to what it was before that concept existed.
alter table channels drop constraint if exists channels_kind_check;
alter table channels
  add constraint channels_kind_check
  check (kind in ('text', 'voice', 'video', 'announcement', 'private', 'dm'));
