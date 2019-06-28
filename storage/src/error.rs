#[derive(Debug, PartialEq, Display)]
pub enum Error {
    /// Low level database error
    #[display(fmt = "Database error: {}", _0)]
    DatabaseError(String),
    /// Invalid block
    #[display(fmt = "Cannot canonize block")]
    CannotCanonize,
    /// Unknown parent
    #[display(fmt = "Block parent is unknown")]
    UnknownParent,
    /// Ancient fork
    #[display(fmt = "Fork is too long to proceed")]
    AncientFork,
    /// Invalid block
    #[display(fmt = "Cannot decanonize block (invalid database state)")]
    CannotDecanonize,
}

impl From<Error> for String {
    fn from(e: Error) -> String {
        format!("{}", e)
    }
}
