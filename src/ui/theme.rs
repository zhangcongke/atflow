use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    Mist,
    Ink,
    Paper,
}

impl Default for ThemeName {
    fn default() -> Self {
        Self::Mist
    }
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
