-- User profile fields (optional)

alter table users add column if not exists name text;
alter table users add column if not exists avatar_url text;

