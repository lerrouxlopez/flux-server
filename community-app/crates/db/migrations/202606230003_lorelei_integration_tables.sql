-- Bridges FLUX orgs/users/channels to a Lorelei tenant+agent (see LORELEI_BUILDPLAN.md
-- in the flux frontend repo for the full design).

create table if not exists org_lorelei_settings (
  organization_id uuid primary key references organizations(id) on delete cascade,
  enabled boolean not null default false,
  display_name text not null default 'Lorelei',
  default_provider text not null default 'ollama',
  default_model text not null default 'llama3.2:3b',
  bot_user_id uuid references users(id),
  lorelei_tenant_id uuid,
  lorelei_agent_id uuid,
  default_channel_id uuid references channels(id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

do $$
begin
  alter table org_lorelei_settings
    add constraint org_lorelei_settings_provider_check
    check (default_provider in ('ollama', 'openai', 'anthropic'));
exception
  when duplicate_object then null;
end $$;

create table if not exists org_lorelei_channels (
  id uuid primary key default gen_random_uuid(),
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid not null references channels(id) on delete cascade,
  enabled_by uuid not null references users(id),
  created_at timestamptz not null default now(),
  unique (organization_id, channel_id)
);

create index if not exists org_lorelei_channels_org_idx on org_lorelei_channels (organization_id);

create table if not exists user_llm_credentials (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  provider text not null,
  encrypted_api_key bytea not null,
  key_fingerprint text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (user_id, provider)
);

do $$
begin
  alter table user_llm_credentials
    add constraint user_llm_credentials_provider_check
    check (provider in ('openai', 'anthropic'));
exception
  when duplicate_object then null;
end $$;

create table if not exists user_lorelei_preferences (
  user_id uuid primary key references users(id) on delete cascade,
  preferred_provider text,
  preferred_model text,
  updated_at timestamptz not null default now()
);

do $$
begin
  alter table user_lorelei_preferences
    add constraint user_lorelei_preferences_provider_check
    check (preferred_provider is null or preferred_provider in ('ollama', 'openai', 'anthropic'));
exception
  when duplicate_object then null;
end $$;
