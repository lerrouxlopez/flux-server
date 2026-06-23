-- Lorelei becomes a single global platform user rather than one bot-user-per-org. Each
-- FLUX user can PM her directly (reusing the existing DM mechanism); org admins/owners can
-- additionally add her to specific channels. See LORELEI_BUILDPLAN.md Section 0 for the
-- full redesign rationale.
insert into users (id, email, display_name, password_hash, is_system_bot)
values ('63dcae57-b2f5-4725-a161-c13599113a80', 'lorelei@system.flux.internal', 'Lorelei', null, true)
on conflict (id) do nothing;
