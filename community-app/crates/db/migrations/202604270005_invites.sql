create table organization_invites (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  token text not null unique,
  role text not null default 'member',
  created_by uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  expires_at timestamptz,
  used_at timestamptz,
  used_by uuid references users(id) on delete set null
);

create index organization_invites_org_idx on organization_invites (organization_id, created_at desc);
