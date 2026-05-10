alter table channels
  add column if not exists created_by uuid references users(id) on delete set null;

create index if not exists channels_created_by_idx on channels (created_by);

