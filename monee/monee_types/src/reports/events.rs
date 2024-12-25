

pub mod event {
    use monee_core::Amount;

    use crate::{
        backoffice::{
            actors::actor::Actor, currencies::currency::Currency,
            wallets::wallet_name::WalletName,
        },
        shared::date::Datetime,
    };

    #[derive(serde::Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Event {
        Purchase {
            item: String,
            actors: Box<[Actor]>,
            wallet: WalletName,
            amount: Amount,
        },
        MoveValue {
            from: WalletName,
            to: WalletName,
            amount: Amount,
        },
        RegisterBalance {
            wallet: WalletName,
            amount: Amount,
        },

        RegisterDebt(DebtRegister),
        RegisterLoan(DebtRegister),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct DebtRegister {
        pub amount: Amount,
        pub currency: Currency,
        pub actor: Actor,
        pub payment_promise: Option<Datetime>,
    }
}

