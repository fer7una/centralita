use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn now_iso() -> Result<String, time::error::Format> {
    OffsetDateTime::now_utc().format(&Rfc3339)
}
