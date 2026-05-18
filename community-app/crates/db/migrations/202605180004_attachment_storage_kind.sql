-- Attachment storage abstraction: distinguish legacy inline data URLs from stored objects.

alter table message_attachments
  add column if not exists storage_kind text not null default 'data_url';

-- Backfill obvious local fs keys (anything not starting with "data:").
update message_attachments
set storage_kind = 'local_fs'
where storage_path not like 'data:%';

create index if not exists message_attachments_kind_idx
  on message_attachments (storage_kind, created_at desc);

