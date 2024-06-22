pub use currency_id_or_code::CurrencyIdOrCode;
mod currency_id_or_code {
    use std::str::FromStr;

    #[derive(Clone)]
    pub enum CurrencyIdOrCode {
        Id(twon_core::CurrencyId),
        Code(String),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        InvalidId(<twon_core::CurrencyId as FromStr>::Err),
        #[error("Length must be 3 or 4")]
        InvalidLength,
    }

    impl FromStr for CurrencyIdOrCode {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() == 4 {
                let id = twon_core::CurrencyId::from_str(s).map_err(Error::InvalidId)?;
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
    con: &twon_persistence::database::Connection,
    currency: CurrencyIdOrCode,
    yes: bool,
) -> miette::Result<Option<twon_core::CurrencyId>> {
    let currency_id = match currency {
        CurrencyIdOrCode::Id(currency_id) => {
            let exists =
                match twon_persistence::actions::check_currency_id::run(con, currency_id).await {
                    Ok(exists) => exists,
                    Err(err) => twon_persistence::log::database(err),
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
            use twon_persistence::actions::currency_id_from_code;
            match twon_persistence::actions::currency_id_from_code::run(con, code.clone()).await {
                Ok(id) => id,
                Err(currency_id_from_code::Error::NotFound) => {
                    let diagnostic = miette::diagnostic!(
                        severity = miette::Severity::Error,
                        code = "currency::NotFound",
                        "Currency with code `{}` not found",
                        code
                    );

                    return Err(diagnostic.into());
                }
                Err(currency_id_from_code::Error::Database(error)) => {
                    twon_persistence::log::database(error)
                }
            }
        }
    };

    Ok(Some(currency_id))
}
