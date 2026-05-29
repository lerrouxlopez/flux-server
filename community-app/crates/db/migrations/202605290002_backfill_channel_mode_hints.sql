-- Bring existing orgs in line with the new default channel spec:
--   Work mode:  General (text, 'work'), Announcements (announcement, NULL), Reports (text, 'work')
--   Play mode:  General (text, 'play'), Announcements (announcement, NULL), Voice (voice, 'play')
--
-- Announcements has no hint so it appears in both modes.

-- 1. Remove any wrong 'Announcement' (singular) default channels.
DELETE FROM channels
WHERE created_by IS NULL AND name = 'Announcement' AND kind = 'announcement';

-- 2. Announcements should have no mode hint (shows in both modes).
UPDATE channels
SET experience_mode_hint = NULL
WHERE created_by IS NULL AND name = 'Announcements' AND kind = 'announcement';

-- 3. Reports is work-only.
UPDATE channels
SET experience_mode_hint = 'work'
WHERE created_by IS NULL AND name = 'Reports';

-- 4. Voice is play-only.
UPDATE channels
SET experience_mode_hint = 'play'
WHERE created_by IS NULL AND name = 'Voice' AND kind = 'voice';

-- 5. Assign General hints: earliest id per org → work, second → play.
WITH ranked AS (
  SELECT id, ROW_NUMBER() OVER (PARTITION BY organization_id ORDER BY id) AS rn
  FROM channels
  WHERE created_by IS NULL AND name = 'General' AND kind = 'text'
)
UPDATE channels
SET experience_mode_hint = CASE WHEN ranked.rn = 1 THEN 'work' ELSE 'play' END
FROM ranked
WHERE channels.id = ranked.id;

-- 6. Insert any missing default channels for orgs created before this spec.

INSERT INTO channels (id, organization_id, name, kind, experience_mode_hint, created_at)
SELECT gen_random_uuid(), o.id, 'Announcements', 'announcement', NULL, clock_timestamp()
FROM organizations o
WHERE NOT EXISTS (
  SELECT 1 FROM channels c
  WHERE c.organization_id = o.id AND c.name = 'Announcements' AND c.kind = 'announcement' AND c.created_by IS NULL
);

INSERT INTO channels (id, organization_id, name, kind, experience_mode_hint, created_at)
SELECT gen_random_uuid(), o.id, 'Reports', 'text', 'work', clock_timestamp()
FROM organizations o
WHERE NOT EXISTS (
  SELECT 1 FROM channels c
  WHERE c.organization_id = o.id AND c.name = 'Reports' AND c.created_by IS NULL
);

INSERT INTO channels (id, organization_id, name, kind, experience_mode_hint, created_at)
SELECT gen_random_uuid(), o.id, 'Voice', 'voice', 'play', clock_timestamp()
FROM organizations o
WHERE NOT EXISTS (
  SELECT 1 FROM channels c
  WHERE c.organization_id = o.id AND c.name = 'Voice' AND c.kind = 'voice' AND c.created_by IS NULL
);

INSERT INTO channels (id, organization_id, name, kind, experience_mode_hint, created_at)
SELECT gen_random_uuid(), o.id, 'General', 'text', 'work', clock_timestamp()
FROM organizations o
WHERE NOT EXISTS (
  SELECT 1 FROM channels c
  WHERE c.organization_id = o.id AND c.name = 'General' AND c.kind = 'text'
    AND c.experience_mode_hint = 'work' AND c.created_by IS NULL
);

INSERT INTO channels (id, organization_id, name, kind, experience_mode_hint, created_at)
SELECT gen_random_uuid(), o.id, 'General', 'text', 'play', clock_timestamp()
FROM organizations o
WHERE NOT EXISTS (
  SELECT 1 FROM channels c
  WHERE c.organization_id = o.id AND c.name = 'General' AND c.kind = 'text'
    AND c.experience_mode_hint = 'play' AND c.created_by IS NULL
);
