use monee::prelude::AppContext;
use std::{fmt::Display, str::FromStr};

use crate::error::PanicError;

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
            Err(err) => Err(PanicError::new(err).into_final_report(ctx)),
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

mod impl_trait {
    use cream::context::Context;
    use monee::{
        backoffice::{
            actors::domain::actor_alias::ActorAlias,
            currencies::domain::currency_code::CurrencyCode,
            item_tags::domain::item_name::ItemName, wallets::domain::wallet_name::WalletName,
        },
        prelude::AppContext,
    };
    use monee_core::{ActorId, CurrencyId, ItemTagId, WalletId};

    use super::AliasedId;

    impl AliasedId for CurrencyId {
        type Alias = CurrencyCode;

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

    impl AliasedId for WalletId {
        type Alias = WalletName;

        async fn resolve(
            ctx: &AppContext,
            alias: Self::Alias,
        ) -> Result<Option<Self>, monee::shared::infrastructure::errors::InfrastructureError>
        {
            let service =
                ctx.provide::<monee::backoffice::wallets::application::name_resolve::NameResolve>();
            service.run(&alias).await
        }
    }

    impl AliasedId for ActorId {
        type Alias = ActorAlias;
        async fn resolve(
            ctx: &AppContext,
            alias: Self::Alias,
        ) -> Result<Option<Self>, monee::shared::infrastructure::errors::InfrastructureError>
        {
            let service = ctx
                .provide::<monee::backoffice::actors::application::alias_resolve::AliasResolve>();
            service.run(&alias).await
        }
    }

    impl AliasedId for ItemTagId {
        type Alias = ItemName;
        async fn resolve(
            ctx: &AppContext,
            alias: Self::Alias,
        ) -> Result<Option<Self>, monee::shared::infrastructure::errors::InfrastructureError>
        {
            let service = ctx
                .provide::<monee::backoffice::item_tags::application::name_resolve::NameResolve>();
            service.run(&alias).await
        }
    }
}
