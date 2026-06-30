use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    Mist,
    Ink,
    Paper,
}

impl ThemeName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mist => "mist",
            Self::Ink => "ink",
            Self::Paper => "paper",
        }
    }
}
