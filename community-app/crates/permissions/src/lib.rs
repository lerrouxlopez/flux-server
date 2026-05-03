pub type Perms = i64;

pub mod perms {
    use super::Perms;

    // --- Organization / admin ---
    pub const ORG_MANAGE: Perms = 1 << 0;
    pub const ORG_MANAGE_MEMBERS: Perms = 1 << 1;
    pub const BRANDING_MANAGE: Perms = 1 << 2;
    pub const ADMIN_AUDIT_LOG_VIEW: Perms = 1 << 3;
    pub const ORG_INVITES_CREATE: Perms = 1 << 4;

    // --- Channels ---
    pub const CHANNELS_VIEW: Perms = 1 << 10;
    pub const CHANNELS_CREATE: Perms = 1 << 11;
    pub const CHANNELS_MANAGE: Perms = 1 << 12;

    // --- Messages ---
    pub const MESSAGES_SEND: Perms = 1 << 20;
    pub const MESSAGES_EDIT_OWN: Perms = 1 << 21;
    pub const MESSAGES_DELETE_OWN: Perms = 1 << 22;
    pub const MESSAGES_DELETE_ANY: Perms = 1 << 23;
    pub const MESSAGES_REACT: Perms = 1 << 24;

    // --- Media rooms ---
    pub const MEDIA_ROOMS_CREATE: Perms = 1 << 28;

    pub const VOICE_JOIN: Perms = 1 << 30;
    pub const VOICE_SPEAK: Perms = 1 << 31;
    pub const VIDEO_START: Perms = 1 << 32;
    pub const SCREEN_SHARE: Perms = 1 << 33;

    // Back-compat aliases (older code)
    pub const ORGS_MANAGE: Perms = ORG_MANAGE;
    pub const ORGS_MEMBERS_MANAGE: Perms = ORG_MANAGE_MEMBERS;
    pub const ORGS_INVITES_CREATE: Perms = ORG_INVITES_CREATE;
    pub const MESSAGES_MANAGE: Perms = MESSAGES_DELETE_ANY;

    pub const ALL: Perms = i64::MAX;
}

#[inline]
pub fn has(perms: Perms, needed: Perms) -> bool {
    (perms & needed) == needed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    OrgManage,
    OrgManageMembers,
    BrandingManage,
    AdminAuditLogView,
    OrgInvitesCreate,

    ChannelsView,
    ChannelsCreate,
    ChannelsManage,

    MessagesSend,
    MessagesEditOwn,
    MessagesDeleteOwn,
    MessagesDeleteAny,
    MessagesReact,

    MediaRoomsCreate,

    VoiceJoin,
    VoiceSpeak,
    VideoStart,
    ScreenShare,
}

impl Permission {
    pub fn bit(self) -> Perms {
        use perms::*;
        match self {
            Permission::OrgManage => ORG_MANAGE,
            Permission::OrgManageMembers => ORG_MANAGE_MEMBERS,
            Permission::BrandingManage => BRANDING_MANAGE,
            Permission::AdminAuditLogView => ADMIN_AUDIT_LOG_VIEW,
            Permission::OrgInvitesCreate => ORG_INVITES_CREATE,

            Permission::ChannelsView => CHANNELS_VIEW,
            Permission::ChannelsCreate => CHANNELS_CREATE,
            Permission::ChannelsManage => CHANNELS_MANAGE,

            Permission::MessagesSend => MESSAGES_SEND,
            Permission::MessagesEditOwn => MESSAGES_EDIT_OWN,
            Permission::MessagesDeleteOwn => MESSAGES_DELETE_OWN,
            Permission::MessagesDeleteAny => MESSAGES_DELETE_ANY,
            Permission::MessagesReact => MESSAGES_REACT,

            Permission::MediaRoomsCreate => MEDIA_ROOMS_CREATE,

            Permission::VoiceJoin => VOICE_JOIN,
            Permission::VoiceSpeak => VOICE_SPEAK,
            Permission::VideoStart => VIDEO_START,
            Permission::ScreenShare => SCREEN_SHARE,
        }
    }
}
