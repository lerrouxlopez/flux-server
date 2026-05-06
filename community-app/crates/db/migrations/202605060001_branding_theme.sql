-- Add theme mode to branding profiles (dark/light).
alter table branding_profiles
  add column if not exists theme text not null default 'dark';

do $$
begin
  alter table branding_profiles
    add constraint branding_profiles_theme_check
    check (theme in ('dark','light'));
exception
  when duplicate_object then null;
end $$;

