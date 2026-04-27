pub fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} must be set"))
}

pub fn optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

pub fn parse<T>(key: &str) -> anyhow::Result<Option<T>>
where
    T: std::str::FromStr,
{
    let Some(v) = optional(key) else {
        return Ok(None);
    };
    v.parse::<T>()
        .map(Some)
        .map_err(|_| anyhow::anyhow!("{key} is invalid"))
}

