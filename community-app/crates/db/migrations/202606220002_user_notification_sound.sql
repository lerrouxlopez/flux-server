-- User's chosen notification sound (one of 10 client-side synthesized tones).
-- No constraint on value -- an unknown id falls back to the client default.
alter table users add column if not exists notification_sound text;
