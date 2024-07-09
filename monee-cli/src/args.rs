pub mod alias {
    use std::str::FromStr;

    use monee_core::alias;

    #[derive(Debug, Clone)]
    pub enum Arg<I: AliasedId> {
        Id(I),
        Alias(alias::Alias),
    }

    pub async fn get_id<I: AliasedId>(
        db: &monee::database::Connection,
        id: Arg<I>,
    ) -> miette::Result<I> {
        match id {
            Arg::Id(id) => Ok(id),
            Arg::Alias(alias) => match I::get_id(db, &alias).await {
                Ok(Some(id)) => Ok(id),
                Ok(None) => {
                    let code = format!("{}::NotFound", I::ENTITY_NAME);
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = code,
                        "{} with {} `{}` not found",
                        I::ENTITY_NAME,
                        I::ALIAS_FIELD,
                        alias
                    );

                    Err(diagnostic.into())
                }
                Err(err) => monee::log::database(err),
            },
        }
    }

    pub trait AliasedId: FromStr + Sized {
        const ENTITY_NAME: &'static str;
        const ALIAS_FIELD: &'static str = "alias";

        async fn get_id(
            db: &monee::database::Connection,
            alias: &alias::Alias,
        ) -> monee::database::Result<Option<Self>>;
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error<I: AliasedId>
    where
        I::Err: std::error::Error,
    {
        #[error(transparent)]
        Id(<I as FromStr>::Err),
        #[error(transparent)]
        Alias(alias::from_str::Error),
    }

    impl<I: AliasedId> FromStr for Arg<I>
    where
        I::Err: std::error::Error,
    {
        type Err = Error<I>;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if let Some(raw) = s.strip_prefix(I::ENTITY_NAME) {
                if let Some(raw) = raw.strip_prefix(':') {
                    let id = I::from_str(raw).map_err(Error::Id)?;
                    return Ok(Arg::Id(id));
                }
            }

            let alias = alias::Alias::from_str(s).map_err(Error::Alias)?;
            Ok(Arg::Alias(alias))
        }
    }

    impl AliasedId for monee_core::WalletId {
        const ENTITY_NAME: &'static str = "wallet";
        const ALIAS_FIELD: &'static str = "name";

        async fn get_id(
            db: &monee::database::Connection,
            name: &alias::Alias,
        ) -> monee::database::Result<Option<Self>> {
            monee::actions::wallets::alias_get::run(db, name.as_str()).await
        }
    }

    impl AliasedId for monee_core::actor::ActorId {
        const ENTITY_NAME: &'static str = "actor";
        const ALIAS_FIELD: &'static str = "alias";

        async fn get_id(
            db: &monee::database::Connection,
            alias: &alias::Alias,
        ) -> monee::database::Result<Option<Self>> {
            monee::actions::actors::alias_get::run(db, alias.as_str()).await
        }
    }
}

pub use currency_id_or_code::CurrencyIdOrCode;
mod currency_id_or_code {
    use std::str::FromStr;

    #[derive(Clone)]
    pub enum CurrencyIdOrCode {
        Id(monee_core::CurrencyId),
        Code(String),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        InvalidId(<monee_core::CurrencyId as FromStr>::Err),
        #[error("Length must be 3 or 4")]
        InvalidLength,
    }

    impl FromStr for CurrencyIdOrCode {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() == 4 {
                let id = monee_core::CurrencyId::from_str(s).map_err(Error::InvalidId)?;
                return Ok(CurrencyIdOrCode::Id(id));
            }

            if s.len() == 3 {
                return Ok(CurrencyIdOrCode::Code(s.to_owned()));
            }

            Err(Error::InvalidLength)
        }
    }
}

async fn confirm_continue() -> bool {
    use tokio::io::AsyncBufReadExt;
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();

    if let Err(why) = stdin.read_line(&mut line).await {
        panic!("Failed to read line: {}", why);
    }

    let answer = line.trim().to_lowercase();
    answer.is_empty() || answer == "y" || answer == "yes"
}

pub async fn get_currency(
    con: &monee::database::Connection,
    currency: CurrencyIdOrCode,
    yes: bool,
) -> miette::Result<Option<monee_core::CurrencyId>> {
    let currency_id = match currency {
        CurrencyIdOrCode::Id(currency_id) => {
            let exists = match monee::actions::currencies::check::run(con, currency_id).await {
                Ok(exists) => exists,
                Err(err) => monee::log::database(err),
            };

            if !exists && !yes {
                use tokio::io::AsyncWriteExt;

                let buf = format!("Currency `{}` not found, continue? (Y/n) ", currency_id);

                let mut stdout = tokio::io::stdout();
                stdout.write_all(buf.as_bytes()).await.expect("To write");
                stdout.flush().await.expect("To flush");

                let should_continue = confirm_continue().await;

                if !should_continue {
                    return Ok(None);
                }
            }

            currency_id
        }
        CurrencyIdOrCode::Code(code) => {
            use monee::actions::currencies::from_code;
            match from_code::run(con, code.clone()).await {
                Ok(id) => id,
                Err(from_code::Error::NotFound) => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "currency::NotFound",
                        "Currency with code `{}` not found",
                        code
                    );

                    return Err(diagnostic.into());
                }
                Err(from_code::Error::Database(error)) => monee::log::database(error),
            }
        }
    };

    Ok(Some(currency_id))
}
