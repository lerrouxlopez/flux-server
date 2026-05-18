-- Experience mode resolver fields (Work/Play).

alter table organizations
  add column if not exists experience_default_mode text not null default 'work';

do $$
begin
  alter table organizations
    add constraint organizations_experience_default_mode_check
    check (experience_default_mode in ('work','play'));
exception
  when duplicate_object then null;
end $$;

alter table users
  add column if not exists experience_mode_preference text;

do $$
begin
  alter table users
    add constraint users_experience_mode_preference_check
    check (experience_mode_preference is null or experience_mode_preference in ('work','play'));
exception
  when duplicate_object then null;
end $$;

alter table channels
  add column if not exists experience_mode_hint text;

do $$
begin
  alter table channels
    add constraint channels_experience_mode_hint_check
    check (experience_mode_hint is null or experience_mode_hint in ('work','play'));
exception
  when duplicate_object then null;
end $$;

create index if not exists channels_experience_mode_hint_idx
  on channels (organization_id, experience_mode_hint)
  where experience_mode_hint is not null;

