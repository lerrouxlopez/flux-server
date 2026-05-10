alter table branding_profiles
  add column if not exists selection_bg text,
  add column if not exists selection_text text,
  add column if not exists dropdown_bg text,
  add column if not exists dropdown_text text,
  add column if not exists chat_bubble_me_bg text,
  add column if not exists chat_bubble_me_text text,
  add column if not exists chat_bubble_other_bg text,
  add column if not exists chat_bubble_other_text text;

