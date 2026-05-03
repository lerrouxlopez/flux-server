export type Org = {
  id: string;
  slug: string;
  name: string;
  created_at: string;
};

export type OrgsListResponse = { organizations: Org[] };

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

