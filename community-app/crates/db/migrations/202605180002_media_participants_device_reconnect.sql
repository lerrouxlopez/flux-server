-- Add device-aware identity + reconnect constraints for media participants.

alter table media_participants
  add column if not exists device_id text not null default 'unknown';

-- At most one active participant per session for a given user+device.
create unique index if not exists media_participants_active_session_user_device_uq
  on media_participants (media_session_id, user_id, device_id)
  where left_at is null;

create index if not exists media_participants_device_idx
  on media_participants (device_id, joined_at desc);

