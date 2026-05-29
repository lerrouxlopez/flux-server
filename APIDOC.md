# Flux API Documentation

Frontend developer reference for the Flux REST API.

**Base URL**: Configured via `VITE_BACKEND_ORIGIN` environment variable  
**Content-Type**: `application/json`  
**Authentication**: `Authorization: Bearer <access_token>`

---

## Table of Contents

1. [Authentication](#authentication)
2. [Error Handling](#error-handling)
3. [Organizations](#organizations)
4. [Channels](#channels)
5. [Messages](#messages)
6. [Threads & Pins](#threads--pins)
7. [Media Rooms](#media-rooms)
8. [Friends](#friends)
9. [Direct Messages](#direct-messages)
10. [Branding](#branding)
11. [Experience](#experience)
12. [Notifications](#notifications)
13. [Audit Logs](#audit-logs)
14. [Attachments](#attachments)
15. [Utilities](#utilities)
16. [Permissions Reference](#permissions-reference)

---

## Authentication

Flux uses JWT-based bearer token auth. On login or register, you receive an `access_token` (short-lived) and a `refresh_token` (long-lived). The frontend reads the token from the Zustand auth store, falling back to `localStorage`.

### POST `/auth/register`

Create a new account.

**Request**
```json
{
  "email": "user@example.com",
  "display_name": "Alice",
  "password": "secret123"
}
```

**Response `200`**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ..."
}
```

---

### POST `/auth/login`

Authenticate and receive tokens. Rate-limited to 10 attempts per 60 seconds per email address.

**Request**
```json
{
  "email": "user@example.com",
  "password": "secret123"
}
```

**Response `200`**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ..."
}
```

**Errors**: `rate_limited` (429), `unauthenticated` (401)

---

### POST `/auth/refresh`

Exchange a refresh token for a new access token.

**Request**
```json
{
  "refresh_token": "eyJ..."
}
```

**Response `200`**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ..."
}
```

**Errors**: `unauthenticated` (401)

---

### POST `/auth/logout`

Revoke the current refresh token. Requires auth.

**Response `200`** — empty body

---

### GET `/auth/me`

Get the currently authenticated user. Requires auth.

**Response `200`**
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "name": "Alice Smith",
  "display_name": "Alice",
  "avatar_url": "https://...",
  "created_at": "2024-01-01T00:00:00Z"
}
```

| Field | Type | Notes |
|---|---|---|
| `id` | `string (uuid)` | |
| `email` | `string` | |
| `name` | `string \| null` | Full name, optional |
| `display_name` | `string` | |
| `avatar_url` | `string \| null` | |
| `created_at` | `string (ISO 8601)` | |

---

### PATCH `/auth/me`

Update the current user's profile. Requires auth.

**Request** (all fields optional)
```json
{
  "name": "Alice Smith",
  "display_name": "Alice"
}
```

**Response `200`** — updated `MeResponse` (same shape as GET `/auth/me`)

---

### POST `/auth/me/avatar`

Set avatar from a data URL. Requires auth.

**Request**
```json
{
  "data_url": "data:image/png;base64,..."
}
```

**Response `200`** — updated `MeResponse`

---

## Error Handling

All errors follow a consistent shape:

```json
{
  "error": {
    "code": "not_found",
    "message": "The requested resource was not found."
  }
}
```

| Code | HTTP Status | Default Message |
|---|---|---|
| `validation_error` | 400 | The request was invalid. |
| `unauthenticated` | 401 | Authentication is required. |
| `permission_denied` | 403 | You do not have permission to perform this action. |
| `not_found` | 404 | The requested resource was not found. |
| `conflict` | 409 | The request could not be completed due to a conflict. |
| `rate_limited` | 429 | Too many requests. Please try again later. |
| `internal_error` | 500 | An internal error occurred. |

**Limits**: Requests are capped at 1 MB body size and 10 seconds timeout. Exceeding body size returns `payload_too_large`.

---

## Organizations

### POST `/orgs`

Create a new organization. Requires auth.

**Request**
```json
{
  "name": "My Team",
  "slug": "my-team"
}
```

**Response `200`**
```json
{
  "id": "uuid",
  "slug": "my-team",
  "name": "My Team",
  "created_at": "2024-01-01T00:00:00Z"
}
```

**Errors**: `conflict` if slug is taken.

---

### GET `/orgs`

List organizations the authenticated user belongs to.

**Response `200`**
```json
{
  "organizations": [
    {
      "id": "uuid",
      "slug": "my-team",
      "name": "My Team",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

---

### GET `/orgs/discover`

Public discovery of open organizations. Requires auth.

**Query Parameters**

| Param | Type | Description |
|---|---|---|
| `q` | `string` | Search query |
| `tag` | `string` | Filter by tag |
| `policy` | `string` | Filter by join policy (`open`, `request`, `invite`) |
| `limit` | `number` | Max results |
| `cursor` | `string` | Pagination cursor |

**Response `200`**
```json
{
  "organizations": [
    {
      "id": "uuid",
      "slug": "my-team",
      "name": "My Team",
      "description": "...",
      "avatar_url": "https://...",
      "banner_url": "https://...",
      "join_policy": "open",
      "member_count": 42,
      "online_count": 7,
      "category": "gaming",
      "tags": ["competitive", "casual"],
      "member_count_visible": true,
      "online_count_visible": true
    }
  ],
  "next_cursor": "cursor_string"
}
```

---

### GET `/orgs/{org_id}`

Get a single organization by ID. Requires auth + membership.

**Response `200`** — `OrgResponse` (same shape as create response)

---

### POST `/orgs/join`

Join an org using an invite code. Requires auth.

**Request**
```json
{
  "slug": "my-team",
  "invite_code": "ABC123"
}
```

**Response `200`** — `OrgResponse`

---

### POST `/orgs/{org_id}/join`

Join an open organization directly. Requires auth.

**Response `200`** — `OrgResponse`

---

### POST `/orgs/{org_id}/join-requests`

Submit a join request for a request-gated org. Requires auth.

**Request**
```json
{
  "message": "I'd love to join your community!"
}
```

**Response `200`**
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "status": "pending",
  "message": "I'd love to join!",
  "created_at": "2024-01-01T00:00:00Z",
  "responded_at": null,
  "responded_by": null
}
```

---

### GET `/orgs/{org_id}/join-requests`

List pending join requests. Requires auth + `ORG_MANAGE_MEMBERS` permission.

**Response `200`**
```json
{
  "requests": [
    {
      "id": "uuid",
      "user_id": "uuid",
      "status": "pending",
      "message": "...",
      "created_at": "2024-01-01T00:00:00Z",
      "responded_at": null,
      "responded_by": null
    }
  ]
}
```

---

### POST `/orgs/{org_id}/join-requests/{request_id}/approve`

Approve a join request. Requires auth + `ORG_MANAGE_MEMBERS`.

**Response `200`** — empty body

---

### POST `/orgs/{org_id}/join-requests/{request_id}/reject`

Reject a join request. Requires auth + `ORG_MANAGE_MEMBERS`.

**Response `200`** — empty body

---

### GET `/orgs/{org_id}/discovery-settings`

Get org discovery/branding settings. Requires auth + membership.

**Response `200`**
```json
{
  "discoverable": true,
  "join_policy": "open",
  "description": "A great community",
  "avatar_url": "https://...",
  "banner_url": "https://...",
  "member_count_visible": true,
  "online_count_visible": false,
  "category": "gaming",
  "tags": ["casual"]
}
```

---

### PATCH `/orgs/{org_id}/discovery-settings`

Update discovery settings. Requires auth + `ORG_MANAGE`.

**Request** (all fields optional)
```json
{
  "discoverable": true,
  "join_policy": "request",
  "description": "Updated description",
  "avatar_url": "https://...",
  "banner_url": "https://...",
  "member_count_visible": true,
  "online_count_visible": true,
  "category": "gaming",
  "tags": ["tag1", "tag2"]
}
```

**Response `200`** — updated `DiscoverySettingsResponse`

---

### GET `/orgs/{org_id}/members`

List all members of an organization. Requires auth + `CHANNELS_VIEW`.

**Response `200`**
```json
{
  "members": [
    {
      "user_id": "uuid",
      "email": "user@example.com",
      "display_name": "Alice",
      "role": "member",
      "joined_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

---

### POST `/orgs/{org_id}/members`

Add a member to the organization. Requires auth + `ORG_MANAGE_MEMBERS`.

**Request**
```json
{
  "user_id": "uuid",
  "invite_code": "ABC123"
}
```

**Response `200`** — `MemberResponse`

---

### PATCH `/orgs/{org_id}/members/{user_id}`

Update a member's role. Requires auth + `ORG_MANAGE_MEMBERS`.

**Request**
```json
{
  "role": "moderator"
}
```

**Response `200`** — `MemberResponse`

---

### POST `/orgs/{org_id}/invites`

Create an invite link for the organization. Requires auth + `ORG_INVITES_CREATE`.

**Request** (all fields optional)
```json
{
  "expires_in_seconds": 86400,
  "max_uses": 10
}
```

**Response `200`**
```json
{
  "code": "ABC123",
  "expires_at": "2024-01-02T00:00:00Z",
  "max_uses": 10
}
```

---

### GET `/orgs/{org_id}/roles`

List roles defined for an organization. Requires auth + membership.

**Response `200`**
```json
{
  "roles": [
    {
      "id": "uuid",
      "name": "Admin",
      "permissions": 4398046511103,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

`permissions` is a bitfield integer. See [Permissions Reference](#permissions-reference).

---

## Channels

### GET `/orgs/{org_id}/channels`

List all channels in an organization. Requires auth + `CHANNELS_VIEW`.

**Response `200`**
```json
{
  "channels": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "name": "general",
      "kind": "text",
      "experience_mode_hint": null,
      "created_by": "uuid",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

| Field | Type | Notes |
|---|---|---|
| `kind` | `string` | `"text"`, `"voice"`, `"dm"` |
| `experience_mode_hint` | `string \| null` | UI mode preference for the channel |

---

### POST `/orgs/{org_id}/channels`

Create a new channel. Requires auth + `CHANNELS_CREATE`.

**Request**
```json
{
  "name": "announcements",
  "kind": "text",
  "experience_mode_hint": "focused"
}
```

**Response `200`** — `ChannelResponse`

---

### GET `/channels/{channel_id}`

Get a single channel. Requires auth + `CHANNELS_VIEW`.

**Response `200`** — `ChannelResponse`

---

### PATCH `/channels/{channel_id}`

Update channel name or kind. Requires auth + `CHANNELS_MANAGE`.

**Request** (all fields optional)
```json
{
  "name": "new-name",
  "kind": "text"
}
```

**Response `200`** — `ChannelResponse`

---

### DELETE `/channels/{channel_id}`

Delete a channel. Requires auth + `CHANNELS_MANAGE`.

**Response `200`** — empty body

---

## Messages

### GET `/channels/{channel_id}/messages`

List messages in a channel (newest-first, cursor pagination). Requires auth + `CHANNELS_VIEW`.

**Query Parameters**

| Param | Type | Description |
|---|---|---|
| `limit` | `number` | Max messages to return (default varies) |
| `before` | `string` | Cursor — return messages before this cursor |

**Response `200`**
```json
{
  "messages": [
    {
      "id": "uuid",
      "channel_id": "uuid",
      "organization_id": "uuid",
      "author_id": "uuid",
      "author_display_name": "Alice",
      "author_avatar_url": "https://...",
      "body": "Hello!",
      "edited_at": null,
      "created_at": "2024-01-01T00:00:00Z",
      "reactions": [
        { "emoji": "👍", "count": 3, "reacted_by_me": true }
      ],
      "attachments": [
        {
          "id": "uuid",
          "filename": "photo.png",
          "content_type": "image/png",
          "size_bytes": 204800,
          "download_url": "https://...",
          "created_at": "2024-01-01T00:00:00Z"
        }
      ]
    }
  ],
  "next_cursor": "cursor_string"
}
```

`next_cursor` is `null` when there are no more messages.

---

### POST `/channels/{channel_id}/messages`

Send a message to a channel. Requires auth + `MESSAGES_SEND`.

**Request**
```json
{
  "body": "Hello, world!",
  "attachments": [
    {
      "filename": "photo.png",
      "content_type": "image/png",
      "data_url": "data:image/png;base64,..."
    }
  ]
}
```

`body` and `attachments` are both optional, but at least one should be provided.

**Response `200`** — `MessageResponse`

---

### PATCH `/messages/{message_id}`

Edit a message body. Requires auth + `MESSAGES_EDIT_OWN` (or `MESSAGES_DELETE_ANY` for others' messages).

**Request**
```json
{
  "body": "Updated message text"
}
```

**Response `200`** — updated `MessageResponse`

---

### DELETE `/messages/{message_id}`

Delete a message. Requires auth + `MESSAGES_DELETE_OWN` for own messages, `MESSAGES_DELETE_ANY` for others'.

**Response `200`** — empty body

---

### POST `/messages/{message_id}/reactions`

Add an emoji reaction. Requires auth + `MESSAGES_REACT`.

**Request**
```json
{
  "emoji": "👍"
}
```

**Response `200`** — empty body

---

### DELETE `/messages/{message_id}/reactions/{emoji}`

Remove your emoji reaction. Requires auth + `MESSAGES_REACT`.

`emoji` in the path should be URL-encoded (e.g., `%F0%9F%91%8D` for 👍).

**Response `200`** — empty body

---

## Threads & Pins

### GET `/channels/{channel_id}/threads`

List threads in a channel. Requires auth + `CHANNELS_VIEW`.

**Response `200`**
```json
{
  "threads": [
    {
      "thread": {
        "id": "uuid",
        "organization_id": "uuid",
        "channel_id": "uuid",
        "root_message_id": "uuid",
        "created_by": "uuid",
        "created_at": "2024-01-01T00:00:00Z",
        "last_reply_at": "2024-01-02T00:00:00Z"
      },
      "root": { /* MessageResponse */ },
      "reply_count": 5
    }
  ]
}
```

---

### POST `/channels/{channel_id}/threads`

Create a new thread (either standalone or attached to an existing message). Requires auth + `MESSAGES_SEND`.

**Request**
```json
{
  "body": "Starting a thread here",
  "root_message_id": "uuid"
}
```

Both fields are optional. If `root_message_id` is omitted, a new root message is created.

**Response `200`** — `ThreadResponse`

---

### GET `/threads/{thread_id}`

Get a thread with all its replies. Requires auth + `CHANNELS_VIEW`.

**Response `200`**
```json
{
  "thread": { /* ThreadResponse */ },
  "root": { /* MessageResponse */ },
  "replies": [ /* MessageResponse[] */ ]
}
```

---

### POST `/threads/{thread_id}/replies`

Post a reply to a thread. Requires auth + `MESSAGES_SEND`.

**Request** — same shape as send message (`body`, `attachments`)

**Response `200`** — `MessageResponse`

---

### GET `/channels/{channel_id}/pins`

List pinned messages in a channel. Requires auth + `CHANNELS_VIEW`.

**Response `200`**
```json
{
  "pins": [ /* MessageResponse[] */ ]
}
```

---

### POST `/channels/{channel_id}/pins`

Pin a message. Requires auth + `CHANNELS_MANAGE`.

**Request**
```json
{
  "message_id": "uuid"
}
```

**Response `200`** — empty body

---

### DELETE `/channels/{channel_id}/pins/{message_id}`

Unpin a message. Requires auth + `CHANNELS_MANAGE`.

**Response `200`** — empty body

---

### GET `/channels/{channel_id}/search`

Search messages in a channel. Requires auth + `CHANNELS_VIEW`.

**Query Parameters**

| Param | Type | Description |
|---|---|---|
| `q` | `string` | Search query |

**Response `200`**
```json
{
  "messages": [ /* MessageResponse[] */ ]
}
```

---

## Media Rooms

### POST `/orgs/{org_id}/media/rooms`

Create a media room (voice/video). Requires auth + `MEDIA_ROOMS_CREATE`.

**Request**
```json
{
  "kind": "voice",
  "channel_id": "uuid",
  "name": "Team Call"
}
```

| Field | Type | Notes |
|---|---|---|
| `kind` | `string` | `"voice"` or `"video"` |
| `channel_id` | `string (uuid) \| null` | Optional channel association |
| `name` | `string` | Display name |

**Response `200`**
```json
{
  "id": "uuid",
  "organization_id": "uuid",
  "channel_id": "uuid",
  "livekit_room_name": "lk-room-name",
  "kind": "voice",
  "name": "Team Call",
  "created_by": "uuid",
  "created_at": "2024-01-01T00:00:00Z"
}
```

---

### GET `/media/rooms/{room_id}`

Get media room info. Requires auth + membership in the room's org.

**Response `200`** — `MediaRoomResponse`

---

### POST `/media/rooms/{room_id}/join`

Request to join a media room. Requires auth + `VOICE_JOIN`.

**Response `200`**
```json
{
  "granted": true,
  "token": "livekit_token",
  "session_id": "uuid",
  "room": { /* MediaRoomResponse */ }
}
```

---

### POST `/media/rooms/{room_id}/token`

Issue a LiveKit token for the room. Requires auth + membership.

**Response `200`**
```json
{
  "token": "livekit_token"
}
```

---

### GET `/media/rooms/{room_id}/participants`

List current participants in a room. Requires auth.

**Response `200`**
```json
{
  "participants": [
    {
      "id": "uuid",
      "email": "user@example.com",
      "display_name": "Alice"
    }
  ]
}
```

---

### GET `/media/sessions/{session_id}`

Get status of a media session. Requires auth.

**Response `200`**
```json
{
  "id": "uuid",
  "room_id": "uuid",
  "user_id": "uuid",
  "status": "active",
  "joined_at": "2024-01-01T00:00:00Z"
}
```

---

### POST `/media/sessions/{session_id}/heartbeat`

Keep a session alive. Call periodically (e.g., every 30s). Requires auth.

**Response `200`** — empty body

---

### POST `/media/sessions/{session_id}/leave`

Leave a media session. Requires auth.

**Response `200`** — empty body

---

## Friends

All friend routes are scoped to an organization.

### GET `/orgs/{org_id}/friends`

List friends in this org context. Requires auth + membership.

**Response `200`**
```json
{
  "friends": [
    {
      "id": "uuid",
      "email": "friend@example.com",
      "display_name": "Bob"
    }
  ]
}
```

---

### GET `/orgs/{org_id}/friends/requests`

List incoming and outgoing friend requests. Requires auth + membership.

**Response `200`**
```json
{
  "requests": [
    {
      "id": "uuid",
      "requester": {
        "id": "uuid",
        "email": "alice@example.com",
        "display_name": "Alice"
      },
      "addressee": {
        "id": "uuid",
        "email": "bob@example.com",
        "display_name": "Bob"
      },
      "status": "pending",
      "created_at": "2024-01-01T00:00:00Z",
      "responded_at": null
    }
  ]
}
```

`status` values: `"pending"`, `"accepted"`, `"declined"`

---

### POST `/orgs/{org_id}/friends/requests`

Send a friend request. Requires auth + membership.

**Request**
```json
{
  "user_id": "uuid"
}
```

**Response `200`** — `FriendRequestResponse`

---

### POST `/orgs/{org_id}/friends/requests/{request_id}/accept`

Accept a friend request. Requires auth.

**Response `200`** — empty body

---

### POST `/orgs/{org_id}/friends/requests/{request_id}/decline`

Decline a friend request. Requires auth.

**Response `200`** — empty body

---

### POST `/orgs/{org_id}/friends/requests/{request_id}/cancel`

Cancel a sent friend request. Requires auth.

**Response `200`** — empty body

---

### DELETE `/orgs/{org_id}/friends/{user_id}`

Remove a friend. Requires auth.

**Response `200`** — empty body

---

## Direct Messages

### GET `/orgs/{org_id}/dms`

List all DM conversations in this org. Requires auth + membership.

**Response `200`**
```json
{
  "dms": [
    {
      "channel_id": "uuid",
      "peer": {
        "id": "uuid",
        "email": "friend@example.com",
        "display_name": "Bob"
      }
    }
  ]
}
```

---

### POST `/orgs/{org_id}/dms/{user_id}`

Get or create a DM channel with a user. Idempotent. Requires auth + membership.

**Response `200`** — `DmThread` (same shape as list entry)

---

## Branding

### GET `/public/branding`

Fetch branding config by host (no auth required). Used for white-labeling.

**Query Parameters**

| Param | Type | Required |
|---|---|---|
| `host` | `string` | Yes |

**Response `200`**
```json
{
  "org_id": "uuid",
  "org_name": "My Team",
  "org_slug": "my-team",
  "logo_url": "https://...",
  "favicon_url": "https://...",
  "primary_color": "#6366f1",
  "secondary_color": "#8b5cf6",
  "background_color": "#1e1e2e",
  "surface_color": "#2a2a3e",
  "text_color": "#e2e8f0",
  "accent_color": "#a78bfa",
  "font_family": "Inter",
  "custom_css": null,
  "login_banner_url": "https://...",
  "login_message": "Welcome!",
  "theme_preset": "teams-dark",
  "sidebar_style": "default",
  "density": "comfortable",
  "motion": "full",
  "border_radius": "medium",
  "custom_domain": "chat.myteam.com"
}
```

---

### GET `/orgs/{org_id}/branding`

Get branding config for an org. Requires auth + membership.

**Response `200`** — `BrandingResponse` (same shape as public branding)

---

### PATCH `/orgs/{org_id}/branding`

Update org branding. Requires auth + `BRANDING_MANAGE`.

**Request** — any subset of `BrandingResponse` fields

**Response `200`** — updated `BrandingResponse`

---

### POST `/orgs/{org_id}/branding/preview`

Preview branding changes without saving. Requires auth + `BRANDING_MANAGE`.

**Request** — same as PATCH branding

**Response `200`** — `BrandingResponse` with preview values applied

---

## Experience

### GET `/experience/context`

Fetch the resolved experience context for a user in a given org/channel. Requires auth.

**Query Parameters**

| Param | Type | Required |
|---|---|---|
| `org_id` | `string (uuid)` | Yes |
| `channel_id` | `string (uuid)` | No |

**Response `200`**
```json
{
  "mode": "chat",
  "mode_source": "user_preference",
  "density": "comfortable",
  "motion": "full",
  "notification_profile": "all",
  "media_defaults": {
    "camera": false,
    "microphone": true,
    "screen_share": false
  },
  "feature_flags": {},
  "theme_preference": null
}
```

| Field | Notes |
|---|---|
| `mode` | UI mode: `"chat"`, `"focused"`, `"media"` |
| `mode_source` | Where mode came from: `"user_preference"`, `"channel_hint"`, `"org_default"` |
| `density` | `"compact"`, `"comfortable"`, `"spacious"` |
| `motion` | `"full"`, `"reduced"`, `"none"` |

---

### PATCH `/experience/preferences`

Save user experience preferences. Requires auth.

**Request** (all fields optional)
```json
{
  "mode_preference": { "org_id": "uuid", "mode": "focused" },
  "theme_preference": { "preset": "teams-dark" }
}
```

**Response `200`** — empty body

---

## Notifications

### GET `/notifications/context`

Fetch notification settings for a user in a given org/channel context. Requires auth.

**Query Parameters**

| Param | Type | Required |
|---|---|---|
| `org_id` | `string (uuid)` | Yes |
| `channel_id` | `string (uuid)` | No |

**Response `200`**
```json
{
  "mode": "all",
  "profile_source": "org_default",
  "profile_id": "uuid",
  "behavior": {
    "sound": true,
    "desktop": true,
    "badge": true,
    "mentions_only": false
  }
}
```

---

### PATCH `/notifications/overrides/user`

Set a per-org notification override for the current user. Requires auth.

**Request**
```json
{
  "org_id": "uuid",
  "mode": "mentions",
  "profile_id": "uuid"
}
```

`mode` values: `"all"`, `"mentions"`, `"none"`

**Response `200`** — empty body

---

### PATCH `/notifications/overrides/channel`

Set a per-channel notification override. Requires auth.

**Request**
```json
{
  "channel_id": "uuid",
  "mode": "none",
  "profile_id": "uuid"
}
```

**Response `200`** — empty body

---

## Audit Logs

### GET `/orgs/{org_id}/audit-logs`

List audit log entries. Requires auth + `ADMIN_AUDIT_LOG_VIEW`.

**Query Parameters**

| Param | Type | Description |
|---|---|---|
| `limit` | `number` | Max entries to return |

**Response `200`**
```json
{
  "entries": [
    {
      "id": "uuid",
      "actor": {
        "id": "uuid",
        "email": "admin@example.com",
        "display_name": "Admin"
      },
      "action": "channel.created",
      "target_type": "channel",
      "target_id": "uuid",
      "metadata": {},
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

`actor` is `null` for system-generated events.

---

## Attachments

### GET `/attachments/{attachment_id}/download`

Download an attachment file. Requires auth + access to the message it belongs to.

**Response** — binary file stream with appropriate `Content-Type` header.

Use the `download_url` from a `MessageResponse` attachment — it points to this endpoint.

---

## Utilities

### GET `/healthz`

Liveness check. No auth required.

**Response `200`** — `"ok"`

---

### GET `/readyz`

Readiness check. No auth required.

**Response `200`** — `"ok"`

---

### GET `/metrics`

Prometheus metrics. Not for frontend use.

---

## Permissions Reference

Roles have a `permissions` bitfield. The frontend uses this to gate UI elements.

| Permission | Description |
|---|---|
| `CHANNELS_VIEW` | View channels and messages |
| `CHANNELS_CREATE` | Create channels |
| `CHANNELS_MANAGE` | Rename, delete, pin in channels |
| `MESSAGES_SEND` | Send messages |
| `MESSAGES_EDIT_OWN` | Edit own messages |
| `MESSAGES_DELETE_OWN` | Delete own messages |
| `MESSAGES_DELETE_ANY` | Delete any member's messages |
| `MESSAGES_REACT` | Add/remove emoji reactions |
| `VOICE_JOIN` | Join voice/media rooms |
| `VOICE_SPEAK` | Speak in voice rooms |
| `VIDEO_START` | Enable camera |
| `SCREEN_SHARE` | Share screen |
| `MEDIA_ROOMS_CREATE` | Create voice/video rooms |
| `ORG_MANAGE` | Manage organization settings |
| `ORG_MANAGE_MEMBERS` | Add, remove, update member roles |
| `ORG_INVITES_CREATE` | Generate invite links |
| `BRANDING_MANAGE` | Edit org branding and theme |
| `ADMIN_AUDIT_LOG_VIEW` | View audit log |

### Default Role Permissions

| Role | Key Permissions |
|---|---|
| **Owner** | All permissions |
| **Admin** | ORG_MANAGE + members + channels + messages + media + voice/video/screen + audit |
| **Moderator** | CHANNELS_VIEW + MESSAGES_DELETE_ANY + VOICE_JOIN |
| **Member** | CHANNELS_VIEW + MESSAGES_SEND + MESSAGES_EDIT_OWN + MESSAGES_DELETE_OWN + MESSAGES_REACT + VOICE_JOIN + VIDEO_START + SCREEN_SHARE |
| **Guest** | CHANNELS_VIEW + VOICE_JOIN |

---

## Frontend API Client

The frontend uses a typed `apiFetch` function in `apps/web/src/api/client.ts`:

```typescript
import { apiFetch } from "@/api/client";

// GET with query params
const data = await apiFetch<OrgsListResponse>("/orgs");

// POST with body
const org = await apiFetch<OrgResponse>("/orgs", {
  method: "POST",
  body: JSON.stringify({ name: "My Team", slug: "my-team" }),
});
```

The client automatically:
- Reads the bearer token from the Zustand auth store (falls back to `localStorage`)
- Sets `Content-Type: application/json`
- Sets `Authorization: Bearer <token>`
- Throws an `Error` with the API's `error.message` on non-2xx responses
- Returns parsed JSON on success

All TypeScript types for request/response shapes live in `apps/web/src/api/types.ts`.
