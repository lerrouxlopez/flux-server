-- Lorelei redesign #2: the admin-curated channel allowlist is replaced by a simpler rule —
-- any org owner can @mention her in any regular channel and she'll respond there, using the
-- org's shared memory scope (org_lorelei_settings, unchanged). No more explicit "add her to
-- this channel" admin action, so the allowlist table is gone.
drop table if exists org_lorelei_channels;
