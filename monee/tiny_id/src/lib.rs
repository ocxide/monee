use std::fmt::{Debug, Display};

use rand::{thread_rng, Rng};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TinyId<const N: usize>([u8; N]);

impl<const N: usize> TinyId<N> {
    pub fn new() -> Self {
        let mut id = [0u8; N];
        let mut rng = thread_rng();

        id.fill_with(|| rng.sample(rand::distributions::Alphanumeric));

        Self(id)
    }
}

impl<const N: usize> AsRef<[u8]> for TinyId<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> Debug for TinyId<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TinyId({})", self)
    }
}

impl<const N: usize> Default for TinyId<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Display for TinyId<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{}", *byte as char)?;
        }

        Ok(())
    }
}

mod string {
    use std::fmt;

    use super::TinyId;

    #[derive(Debug)]
    pub enum Error {
        /// Should be alpha-numeric
        Invalid,
        /// Too long
        InvalidLength(usize),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Invalid => write!(f, "invalid"),
                Self::InvalidLength(len) => write!(f, "str should be of length {}", len),
            }
        }
    }

    impl std::error::Error for Error {}

    impl<const N: usize> std::str::FromStr for TinyId<N> {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() != N {
                return Err(Error::InvalidLength(N));
            }

            let mut bytes = [0u8; N];

            let mut chars = s.chars();
            for b in bytes.iter_mut() {
                *b = match chars.next() {
                    None => return Err(Error::InvalidLength(N)),
                    Some(c) if c.is_alphanumeric() => c as u8,
                    _ => return Err(Error::Invalid),
                }
            }

            Ok(Self(bytes))
        }
    }
}

mod slice {
    use super::TinyId;

    #[derive(Debug)]
    pub enum Error {
        NotACII,
        StrError(super::string::Error),
    }

    impl From<super::string::Error> for Error {
        fn from(e: super::string::Error) -> Self {
            Self::StrError(e)
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NotACII => write!(f, "not ASCII"),
                Self::StrError(e) => e.fmt(f),
            }
        }
    }

    impl std::error::Error for Error {}

    impl<const N: usize> TinyId<N> {
        pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
            let Ok(slice) = std::str::from_utf8(slice) else {
                return Err(Error::NotACII);
            };

            slice.parse().map_err(Error::StrError)
        }
    }
}

mod bytes {
    use super::TinyId;

    #[derive(Debug)]
    pub enum Error {
        NotACII,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NotACII => write!(f, "not ASCII"),
            }
        }
    }

    impl std::error::Error for Error {}

    impl<const N: usize> TryFrom<[u8; N]> for TinyId<N> {
        type Error = Error;

        fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
            if value.iter().all(|b| b.is_ascii_alphabetic()) {
                Ok(Self(value))
            } else {
                Err(Error::NotACII)
            }
        }
    }
}

mod sede {
    use serde::de::Error as _;
    use std::{
        fmt::{self, Display},
        marker::PhantomData,
    };

    use super::TinyId;

    impl<const N: usize> serde::Serialize for TinyId<N> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // SAFITY: TinyId<N> is generated with rand::distributions::Alphanumeric
            let str = unsafe { std::str::from_utf8_unchecked(&self.0) };
            serializer.serialize_str(str)
        }
    }

    pub struct DeError<E>(E);

    impl<E: std::error::Error> Display for DeError<E> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Failed to parse TinyId: {}", self.0)
        }
    }

    fn de_error<DeErr: serde::de::Error, Inner: std::error::Error>(e: Inner) -> DeErr {
        DeErr::custom(DeError(e))
    }

    struct TinyIdVisitor<const N: usize>(PhantomData<TinyId<N>>);

    impl<'vi, const N: usize> serde::de::Visitor<'vi> for TinyIdVisitor<N> {
        type Value = TinyId<N>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a TinyId string")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<TinyId<N>, E> {
            value.parse::<TinyId<N>>().map_err(de_error)
        }

        fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<TinyId<N>, E> {
            TinyId::<N>::from_slice(value).map_err(de_error)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<TinyId<N>, A::Error>
        where
            A: serde::de::SeqAccess<'vi>,
        {
            let mut bytes = [0u8; N];
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = match seq.next_element()? {
                    Some(e) => e,
                    None => return Err(A::Error::invalid_length(i, &self)),
                };
            }

            let mut remaining = 0;
            while seq.next_element::<u8>()?.is_some() {
                remaining += 1; 
            }
            
            if remaining > 0 {
                return Err(A::Error::invalid_length(N + remaining, &self))
            }

            match TinyId::<N>::try_from(bytes) {
                Ok(id) => Ok(id),
                Err(e) => Err(de_error(e)),
            }
        }
    }

    impl<'de, const N: usize> serde::Deserialize<'de> for TinyId<N> {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_str(TinyIdVisitor(PhantomData))
        }
    }
}
