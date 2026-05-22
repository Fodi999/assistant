use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkingPlane {
    XZ,
    XY,
    YZ,
}

impl WorkingPlane {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "XZ" => Some(Self::XZ),
            "XY" => Some(Self::XY),
            "YZ" => Some(Self::YZ),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XZ => "XZ",
            Self::XY => "XY",
            Self::YZ => "YZ",
        }
    }
    pub fn accepts_grid(self, gx: i32, gy: i32, gz: i32) -> bool {
        match self {
            Self::XZ => gy == 0,
            Self::XY => gz == 0,
            Self::YZ => gx == 0,
        }
    }
}
