create table organizations (
  id uuid primary key,
  slug text not null unique,
  name text not null,
  created_at timestamptz not null default now()
);

create table users (
  id uuid primary key,
  email text not null unique,
  display_name text not null,
  password_hash text,
  created_at timestamptz not null default now()
);

create table organization_members (
  organization_id uuid not null references organizations(id) on delete cascade,
  user_id uuid not null references users(id) on delete cascade,
  role text not null default 'member',
  joined_at timestamptz not null default now(),
  primary key (organization_id, user_id)
);

create table channels (
  id uuid primary key,
  organization_id uuid not null references organizations(id) on delete cascade,
  name text not null,
  kind text not null,
  created_at timestamptz not null default now()
);

create index channels_org_idx on channels (organization_id);
