//! Тип ошибки геометрического движка.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    InvalidProfile(String),
    InvalidArgument(String),
    InvalidMesh(String),
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeometryError::InvalidProfile(m)  => write!(f, "invalid profile: {m}"),
            GeometryError::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            GeometryError::InvalidMesh(m)     => write!(f, "invalid mesh: {m}"),
        }
    }
}

impl std::error::Error for GeometryError {}
