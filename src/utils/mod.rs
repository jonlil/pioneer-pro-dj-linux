use nom::error::ErrorKind;

pub mod network;

pub fn parse_error<T>(input: T, code: ErrorKind) -> nom::Err<nom::error::Error<T>> {
    nom::Err::Error(nom::error::Error::new(input, code))
}