pub mod app_id {
    use std::fmt::Display;

    use idn::IdN;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, Default, Clone, Copy, PartialEq, Eq)]
    pub struct AppId(IdN<4>);

    impl Display for AppId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::str::FromStr for AppId {
        type Err = <IdN<4> as std::str::FromStr>::Err;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(s.parse()?))
        }
    }
}

pub mod app_name {
    use crate::shared::alias::Alias;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct AppName(Alias);
}

pub mod app_manifest {
    use super::app_name::AppName;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct AppManifest {
        pub name: Option<AppName>,
    }
}
