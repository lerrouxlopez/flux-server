pub type Perms = i64;

pub mod perms {
    use super::Perms;

    pub const CHANNELS_VIEW: Perms = 1 << 0;
    pub const CHANNELS_CREATE: Perms = 1 << 1;
    pub const CHANNELS_MANAGE: Perms = 1 << 2;

    pub const MESSAGES_SEND: Perms = 1 << 10;
    pub const MESSAGES_MANAGE: Perms = 1 << 11;

    pub const ORGS_MANAGE: Perms = 1 << 20;
    pub const ORGS_INVITES_CREATE: Perms = 1 << 21;
    pub const ORGS_MEMBERS_MANAGE: Perms = 1 << 22;

    pub const ALL: Perms = i64::MAX;
}

#[inline]
pub fn has(perms: Perms, needed: Perms) -> bool {
    (perms & needed) == needed
}

