pub mod client {
    use super::client_name::ClientName;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Client {
        pub name: Option<ClientName>,
    }
}

pub mod client_name {
    use crate::shared::alias::Alias;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ClientName(Alias);
}

pub mod client_id {
    use std::fmt::Display;

    use idn::IdN;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, Default, Clone, Copy)]
    pub struct ClientId(IdN<4>);

    impl Display for ClientId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::str::FromStr for ClientId {
        type Err = <IdN<4> as std::str::FromStr>::Err;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(s.parse()?))
        }
    }
}



