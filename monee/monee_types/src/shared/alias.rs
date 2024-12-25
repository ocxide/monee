#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq)]
    pub struct Alias(Box<str>);

    impl AsRef<str> for Alias {
        fn as_ref(&self) -> &str {
            &self.0
        }
    }

    impl std::fmt::Display for Alias {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    pub mod from_str {
        use super::Alias;

        #[derive(Debug)]
        pub enum Error {
            Empty,
            Invalid,
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Empty => write!(f, "Alias cannot be emtpy"),
                    Error::Invalid => write!(
                        f,
                        "Alias must only contain 'a-z', 'A-Z', '0-9', '-', '_' characters"
                    ),
                }
            }
        }

        impl std::error::Error for Error {}

        impl std::str::FromStr for Alias {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s.is_empty() {
                    return Err(Error::Empty);
                }

                let is_valid = s
                    .chars()
                    .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'));

                if is_valid {
                    Ok(Alias(s.into()))
                } else {
                    Err(Error::Invalid)
                }
            }
        }
    }

