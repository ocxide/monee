

pub mod snapshot {
    use std::collections::HashMap;

    use monee_core::{Amount, DebtId, WalletId};

    use crate::backoffice::{
        actors::actor::Actor, currencies::currency::Currency,
        wallets::wallet_name::WalletName,
    };

    #[derive(serde::Deserialize, serde::Serialize)]
    pub struct Snapshot {
        pub wallets: HashMap<WalletId, (Wallet, Money)>,
        pub debts: HashMap<DebtId, (Debt, Money)>,
        pub loans: HashMap<DebtId, (Debt, Money)>,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    pub struct Money {
        pub amount: Amount,
        pub currency: Currency,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    pub struct Debt {
        pub actor: Actor,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    pub struct Wallet {
        pub name: WalletName,
        pub description: String,
    }
}

