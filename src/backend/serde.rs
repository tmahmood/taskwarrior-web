

pub(crate) mod task_status_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(status: &Option<taskchampion::Status>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref d) = *status {
            let status = match d {
                taskchampion::Status::Pending => "pending",
                taskchampion::Status::Completed => "completed",
                taskchampion::Status::Deleted => "deleted",
                taskchampion::Status::Recurring => "recurring",
                taskchampion::Status::Unknown(v) => v.as_ref(),
            };
            return s.serialize_str(status);
        }
        s.serialize_none()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<taskchampion::Status>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(s) = s {
            let t = s.to_lowercase();
            return Ok(Some(match t.as_str() {
                "pending" => taskchampion::Status::Pending,
                "completed" => taskchampion::Status::Completed,
                "deleted" => taskchampion::Status::Deleted,
                "recurring" => taskchampion::Status::Recurring,
                &_ => taskchampion::Status::Unknown(t),
            }));
        }

        Ok(None)
    }
}

pub(crate) mod task_date_format {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y%m%dT%H%M%SZ"; // Is always in UTC, not able to parse %:z

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(dt) = date {
            let s = format!("{}", dt.format(FORMAT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_none()
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(date_str) = s {
            match NaiveDateTime::parse_from_str(&date_str, FORMAT) {
                Ok(dt) => Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}

pub(crate) mod task_date_format_mandatory {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y%m%dT%H%M%SZ"; // Is always in UTC, not able to parse %:z

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date_str = String::deserialize(deserializer)?;
        let date_obj = NaiveDateTime::parse_from_str(&date_str, FORMAT)
            .map_err(serde::de::Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(date_obj, Utc))
    }
}
