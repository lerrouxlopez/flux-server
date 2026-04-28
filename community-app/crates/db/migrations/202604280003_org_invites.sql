-- Organization invites (simple centralized invites; federation later).
create table if not exists organization_invites (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  code text not null unique,
  created_by uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  expires_at timestamptz,
  max_uses integer,
  use_count integer not null default 0
);

create index if not exists organization_invites_org_idx
  on organization_invites (organization_id, created_at desc);

