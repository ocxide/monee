pub mod actor {
    use std::str::FromStr;

    use monee_core::{actor, alias};

    #[derive(Debug, Clone)]
    pub enum Arg {
        Id(actor::ActorId),
        Alias(alias::Alias),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        Id(<actor::ActorId as FromStr>::Err),
        #[error(transparent)]
        Alias(#[from] alias::from_str::Error),
    }

    impl FromStr for Arg {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            const PREFIX: &str = "actor:";
            if s.starts_with(PREFIX) {
                let id = actor::ActorId::from_str(&s[PREFIX.len()..]).map_err(Error::Id)?;
                return Ok(Arg::Id(id));
            }

            let alias = alias::Alias::from_str(s)?;
            Ok(Arg::Alias(alias))
        }
    }

    pub async fn get_id(
        db: &monee::database::Connection,
        id: Arg,
    ) -> miette::Result<actor::ActorId> {
        match id {
            Arg::Id(id) => Ok(id),
            Arg::Alias(alias) => {
                match monee::actions::actors::alias_get::run(db, alias.as_str()).await {
                    Ok(Some(id)) => Ok(id),
                    Ok(None) => {
                        let diagnostic = miette::diagnostic!(
                            severity = miette::Severity::Error,
                            code = "actor::NotFound",
                            "Actor with alias `{}` not found",
                            alias
                        );

                        Err(diagnostic.into())
                    }
                    Err(err) => monee::log::database(err),
                }
            }
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
