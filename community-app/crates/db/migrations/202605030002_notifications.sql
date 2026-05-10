-- Notifications (durable, per-user, org-scoped).
create table if not exists notifications (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  kind text not null,
  message_id uuid references messages(id) on delete set null,
  created_at timestamptz not null default now(),
  read_at timestamptz
);

create index if not exists notifications_user_created_idx
  on notifications (user_id, created_at desc);

create index if not exists notifications_org_created_idx
  on notifications (organization_id, created_at desc);

