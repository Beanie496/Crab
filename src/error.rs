/// An error that occurs when a string cannot be parsed.
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum ParseError {
    /// A token was outside an expected range.
    ErroneousToken,
    /// Expected a token but found nothing.
    ExpectedToken,
    /// Expected a different token.
    InvalidToken,
}
