-- Branding overhaul: presets + tokenized profiles + brand assets.

create table if not exists brand_presets (
  id text primary key,
  ui_mode text not null,
  label text not null,
  description text not null,
  theme text not null, -- 'dark' | 'light'
  tokens jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

do $$
begin
  alter table brand_presets
    add constraint brand_presets_ui_mode_check
    check (ui_mode in ('work','play'));
exception
  when duplicate_object then null;
end $$;

do $$
begin
  alter table brand_presets
    add constraint brand_presets_theme_check
    check (theme in ('dark','light'));
exception
  when duplicate_object then null;
end $$;

create index if not exists brand_presets_mode_idx on brand_presets (ui_mode);

-- Tokenized profiles: store a JSON patch of overrides, plus optional chosen preset.
alter table branding_profiles
  add column if not exists preset_id text references brand_presets(id),
  add column if not exists tokens jsonb not null default '{}'::jsonb;

-- Brand assets (future-friendly: local fs now, object storage later).
create table if not exists brand_assets (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  kind text not null, -- 'logo' | 'icon'
  filename text not null,
  content_type text,
  size_bytes bigint not null,
  storage_kind text not null default 'local_fs',
  storage_path text not null,
  created_by uuid references users(id) on delete set null,
  created_at timestamptz not null default now()
);

do $$
begin
  alter table brand_assets
    add constraint brand_assets_kind_check
    check (kind in ('logo','icon'));
exception
  when duplicate_object then null;
end $$;

create index if not exists brand_assets_org_idx on brand_assets (organization_id, created_at desc);
create index if not exists brand_assets_kind_idx on brand_assets (organization_id, kind, created_at desc);

-- Seed a minimal built-in set of presets (expanded later).
insert into brand_presets (id, ui_mode, label, description, theme, tokens)
values
  (
    'work-01',
    'work',
    'Indigo Focus',
    'Classic slate + indigo primary for clear hierarchy.',
    'dark',
    jsonb_build_object(
      'primary_color', '#4f46e5',
      'secondary_color', '#0f172a',
      'bg_color', '#020617',
      'surface_color', '#0b1220',
      'border_color', '#1f2937',
      'text_color', '#e2e8f0',
      'muted_color', '#94a3b8'
    )
  ),
  (
    'play-01',
    'play',
    'Neon Arcade',
    'Electric cyan + deep slate, energetic but readable.',
    'dark',
    jsonb_build_object(
      'primary_color', '#22d3ee',
      'secondary_color', '#0b1220',
      'bg_color', '#050817',
      'surface_color', '#0b1220',
      'border_color', '#23304a',
      'text_color', '#e2e8f0',
      'muted_color', '#94a3b8'
    )
  )
on conflict (id) do nothing;

