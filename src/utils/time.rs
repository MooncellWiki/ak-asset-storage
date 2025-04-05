use serde::ser::Serializer;
use time::{format_description::well_known::Rfc3339, PrimitiveDateTime};

pub fn serialize_primitive_date_time<S>(
    date: &PrimitiveDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

pub fn serialize_option_primitive_date_time<S>(
    date: &Option<PrimitiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(d) => serialize_primitive_date_time(d, serializer),
        None => serializer.serialize_none(),
    }
}
