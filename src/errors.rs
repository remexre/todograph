use std::error::Error;

/// An error that the `Authorization` header is missing or incorrect.
#[derive(Debug, Display)]
pub struct BadAuth;

impl Error for BadAuth {}

/// A wrapper for `String` that impls `Error`.
#[derive(Debug, Display, From)]
pub struct ErrorString(pub String);

impl Error for ErrorString {}
