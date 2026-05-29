-- User theme preference (decoupled from org branding).
-- No constraint on value — any theme id is valid; unknown ids fall back to default client-side.

alter table users
  add column if not exists experience_theme_preference text;
