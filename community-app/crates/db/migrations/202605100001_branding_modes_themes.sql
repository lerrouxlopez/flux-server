alter table branding_profiles
  add column if not exists ui_mode text not null default 'work',
  add column if not exists ui_theme text not null default 'work-01',
  add column if not exists bg_color text,
  add column if not exists surface_color text,
  add column if not exists text_color text,
  add column if not exists muted_color text,
  add column if not exists border_color text;

do $$
begin
  alter table branding_profiles
    add constraint branding_profiles_ui_mode_check
    check (ui_mode in ('work','play'));
exception
  when duplicate_object then null;
end $$;

