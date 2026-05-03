-- Add a human-friendly room name (stable LiveKit room name remains separate).
alter table media_rooms
  add column if not exists name text not null default '';

