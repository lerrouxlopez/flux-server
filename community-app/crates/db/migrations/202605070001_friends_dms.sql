-- Friends + direct messages (DMs)

-- Allow DM channels (per-user visibility)
alter table channels drop constraint if exists channels_kind_check;
alter table channels
  add constraint channels_kind_check
  check (kind in ('text', 'voice', 'announcement', 'private', 'dm'));

-- Friend requests are organization-scoped (only org members can friend each other).
create table if not exists friend_requests (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  requester_id uuid not null references users(id) on delete cascade,
  addressee_id uuid not null references users(id) on delete cascade,
  status text not null,
  created_at timestamptz not null default now(),
  responded_at timestamptz
);

do $$
begin
  alter table friend_requests
    add constraint friend_requests_status_check
    check (status in ('pending', 'accepted', 'declined', 'cancelled'));
exception
  when duplicate_object then null;
end $$;

do $$
begin
  alter table friend_requests
    add constraint friend_requests_no_self_check
    check (requester_id <> addressee_id);
exception
  when duplicate_object then null;
end $$;

create index if not exists friend_requests_org_idx
  on friend_requests (organization_id, created_at desc);

create index if not exists friend_requests_addressee_idx
  on friend_requests (organization_id, addressee_id, created_at desc);

create index if not exists friend_requests_requester_idx
  on friend_requests (organization_id, requester_id, created_at desc);

-- Prevent duplicate pending requests in the same direction.
create unique index if not exists friend_requests_pending_uq
  on friend_requests (organization_id, requester_id, addressee_id)
  where status = 'pending';

-- DM channel membership (only members listed can access DM channels).
create table if not exists dm_channel_members (
  channel_id uuid not null references channels(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  added_at timestamptz not null default now(),
  primary key (channel_id, user_id)
);

create index if not exists dm_channel_members_user_idx
  on dm_channel_members (user_id);

