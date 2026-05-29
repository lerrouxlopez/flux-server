alter table channels drop constraint if exists channels_kind_check;
alter table channels
  add constraint channels_kind_check
  check (kind in ('text', 'voice', 'video', 'announcement', 'private', 'dm'));
