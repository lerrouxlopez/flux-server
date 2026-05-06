export type Org = {
  id: string;
  slug: string;
  name: string;
  created_at: string;
};

export type OrgsListResponse = { organizations: Org[] };

export type CreateOrgRequest = { name: string; slug: string };

export type Channel = {
  id: string;
  organization_id: string;
  name: string;
  kind: string;
  created_at: string;
};

export type ChannelsResponse = { channels: Channel[] };

export type Message = {
  id: string;
  organization_id: string;
  channel_id: string;
  sender_id: string;
  body: string | null;
  kind: string;
  created_at: string;
  edited_at?: string | null;
  deleted_at?: string | null;
};

export type ListMessagesResponse = { messages: Message[]; next_cursor?: string | null };

export type MediaRoom = {
  id: string;
  organization_id: string;
  channel_id?: string | null;
  livekit_room_name: string;
  kind: string;
  name: string;
  created_by: string;
  created_at: string;
};

export type TokenResponse = { token: string; livekit_url: string };

export type Member = { user_id: string; email: string; display_name: string; role: string; joined_at: string };
export type MembersResponse = { members: Member[] };

export type InviteResponse = { code: string; expires_at?: string | null; max_uses?: number | null };

export type Role = { id: string; name: string; permissions: number; created_at: string };
export type RolesResponse = { roles: Role[] };

export type Branding = {
  organization_id: string;
  app_name: string;
  logo_url?: string | null;
  icon_url?: string | null;
  primary_color?: string | null;
  secondary_color?: string | null;
  privacy_url?: string | null;
  terms_url?: string | null;
  updated_at: string;
};

export type AuditLogEntry = {
  id: string;
  actor?: { id: string; email: string; display_name: string } | null;
  action: string;
  target_type?: string | null;
  target_id?: string | null;
  metadata: unknown;
  created_at: string;
};
export type AuditLogsResponse = { entries: AuditLogEntry[] };
