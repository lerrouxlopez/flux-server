-- Threads, per-channel pins, and basic search support.

create table if not exists threads (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid not null references channels(id) on delete cascade,
  root_message_id uuid not null references messages(id) on delete cascade,
  created_by uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  last_reply_at timestamptz
);

create unique index if not exists threads_root_message_uq on threads (root_message_id);
create index if not exists threads_channel_idx on threads (channel_id, created_at desc);
create index if not exists threads_org_idx on threads (organization_id, created_at desc);

alter table messages
  add column if not exists thread_id uuid references threads(id) on delete cascade;

create index if not exists messages_thread_created_idx
  on messages (thread_id, created_at desc, id desc)
  where thread_id is not null;

create table if not exists channel_pins (
  organization_id uuid not null references organizations(id) on delete cascade,
  channel_id uuid not null references channels(id) on delete cascade,
  message_id uuid not null references messages(id) on delete cascade,
  pinned_by uuid not null references users(id) on delete cascade,
  pinned_at timestamptz not null default now(),
  primary key (channel_id, message_id)
);

create index if not exists channel_pins_channel_idx on channel_pins (channel_id, pinned_at desc);
create index if not exists channel_pins_org_idx on channel_pins (organization_id, pinned_at desc);

