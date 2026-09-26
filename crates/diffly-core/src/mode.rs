use std::fmt;

/// Granularity of a text diff.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DiffMode {
    #[default]
    Line,
    Word,
    Char,
}

impl fmt::Display for DiffMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Line => "line",
            Self::Word => "word",
            Self::Char => "char",
        })
    }
}
