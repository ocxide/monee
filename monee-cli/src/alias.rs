use std::{fmt::Display, str::FromStr};

use monee::shared::domain::context::AppContext;
use monee_core::CurrencyId;

pub trait AliasedId: Sized + FromStr + Clone {
    type Alias: FromStr + Display + Clone;

    async fn resolve(
        ctx: &AppContext,
        alias: Self::Alias,
    ) -> Result<Option<Self>, monee::shared::infrastructure::errors::InfrastructureError>;
}

#[derive(Clone)]
pub enum MaybeAlias<I: AliasedId> {
    Alias(I::Alias),
    Id(I),
}

#[allow(private_bounds)]
impl<I> MaybeAlias<I>
where
    I: AliasedId,
{
    pub async fn resolve(self, ctx: &AppContext) -> Result<I, miette::Error> {
        let alias = match self {
            Self::Id(id) => return Ok(id),
            Self::Alias(alias) => alias,
        };

        let alias_str = alias.to_string();
        match I::resolve(ctx, alias).await {
            Ok(Some(id)) => Ok(id),
            Ok(None) => Err({
                miette::miette!(code = "NotFound", "Could not resolve alias `{}`", alias_str)
            }),
            Err(err) => Err({
                miette::miette!(
                    code = "InfrastructureError",
                    "Unknown error resolving alias: {}",
                    err
                )
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError<I>
where
    I: AliasedId,
    I::Err: std::error::Error + Send + Sync + 'static,
    <I::Alias as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    Alias(<I::Alias as FromStr>::Err),
    #[error(transparent)]
    Id(<I as FromStr>::Err),
}

impl<I> std::str::FromStr for MaybeAlias<I>
where
    I: AliasedId,
    I::Err: std::error::Error + Send + Sync + 'static,
    <I::Alias as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    type Err = ParseError<I>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(id) = s.strip_prefix("id:") {
            let id = I::from_str(id).map_err(ParseError::Id)?;
            return Ok(Self::Id(id));
        }

        let alias = I::Alias::from_str(s).map_err(ParseError::Alias)?;
        Ok(Self::Alias(alias))
    }
}

impl AliasedId for CurrencyId {
    type Alias = String;

    async fn resolve(
        ctx: &AppContext,
        alias: Self::Alias,
    ) -> Result<Option<Self>, monee::shared::infrastructure::errors::InfrastructureError>
    {
        let service = ctx
            .provide::<monee::backoffice::currencies::application::code_resolve::CodeResolve>();
        service.run(&alias).await
    }
}

