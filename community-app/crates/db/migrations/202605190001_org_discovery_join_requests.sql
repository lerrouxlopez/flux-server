-- Organization discovery + join requests for /orgs gallery.

alter table organizations
  add column if not exists description text,
  add column if not exists avatar_url text,
  add column if not exists banner_url text,
  add column if not exists discoverable boolean not null default false,
  add column if not exists join_policy text not null default 'invite_only',
  add column if not exists member_count_visible boolean not null default true,
  add column if not exists online_count_visible boolean not null default false,
  add column if not exists category text,
  add column if not exists tags text[] not null default '{}';

do $$
begin
  alter table organizations
    add constraint organizations_join_policy_check
    check (join_policy in ('open','invite_only','request','closed'));
exception
  when duplicate_object then null;
end $$;

create index if not exists organizations_discoverable_idx
  on organizations (discoverable, join_policy, created_at desc);

create table if not exists organization_join_requests (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  message text,
  status text not null default 'pending',
  created_at timestamptz not null default now(),
  responded_at timestamptz,
  responded_by uuid references users(id),
  unique (organization_id, user_id)
);

do $$
begin
  alter table organization_join_requests
    add constraint organization_join_requests_status_check
    check (status in ('pending','approved','rejected'));
exception
  when duplicate_object then null;
end $$;

create index if not exists organization_join_requests_org_status_idx
  on organization_join_requests (organization_id, status, created_at desc);

