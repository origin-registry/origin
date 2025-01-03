use serde::de::Error;
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug)]
pub enum DataSize {
    WithUnit(u64, String),
    WithoutUnit(u64),
}

impl DataSize {
    pub fn as_bytes(&self) -> u64 {
        match self {
            Self::WithUnit(size, unit) => match unit.as_str() {
                "K" | "KB" => size * 1000,
                "M" | "MB" => size * 1000 * 1000,
                "G" | "GB" => size * 1000 * 1000 * 1000,
                "KI" | "KIB" => size * 1024,
                "MI" | "MIB" => size * 1024 * 1024,
                "GI" | "GIB" => size * 1024 * 1024 * 1024,
                _ => *size,
            },
            Self::WithoutUnit(size) => *size,
        }
    }
}

// StorageSize is serialized as a string with an optional unit (e.g. "10MB") or as a raw number.
impl<'de> Deserialize<'de> for DataSize {
    fn deserialize<D>(deserializer: D) -> Result<DataSize, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parsed = match s.parse::<u64>() {
            Ok(size) => Self::WithoutUnit(size),
            Err(_) => {
                let size = s
                    .trim_end_matches(|c: char| c.is_alphabetic())
                    .parse::<u64>()
                    .unwrap_or_default();
                let unit = s
                    .trim_start_matches(|c: char| c.is_numeric())
                    .to_uppercase();

                if unit.is_empty() {
                    Self::WithoutUnit(size)
                } else {
                    if ![
                        "K", "KB", "M", "MB", "G", "GB", "KI", "KIB", "MI", "MIB", "GI", "GIB",
                    ]
                    .contains(&unit.as_str())
                    {
                        return Err(D::Error::custom(format!("Invalid unit: {}", unit)));
                    }

                    Self::WithUnit(size, unit)
                }
            }
        };

        Ok(parsed)
    }
}
