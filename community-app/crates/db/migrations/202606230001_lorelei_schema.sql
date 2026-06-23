-- Lorelei's own tables (pearls, currents, runs, tasks, approvals, documents) live in a
-- dedicated schema rather than `public`, so its migrations (run separately via
-- `lorelei-harbor migrate`) can't collide with this database's own table names.
-- The lorelei-harbor/lorelei-cli connection string scopes unqualified table refs here via
-- `?options=-c search_path=lorelei` (see docker-compose.yml).
create schema if not exists lorelei;
grant all on schema lorelei to app;
