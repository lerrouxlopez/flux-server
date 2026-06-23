-- Backfill: Lorelei joins every existing org (new orgs get her added at creation time in
-- routes_orgs.rs::create_org). A member row, nothing more — she has no special permissions.
insert into organization_members (organization_id, user_id, role, joined_at)
select o.id, '63dcae57-b2f5-4725-a161-c13599113a80', 'member', now()
from organizations o
on conflict (organization_id, user_id) do nothing;
