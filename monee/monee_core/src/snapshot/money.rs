use serde::Serialize;

use crate::{Amount, CurrencyId};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Money {
    pub amount: Amount,
    pub currency_id: CurrencyId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoneyMap<M: MoneyHost>(HashMap<M::Key, M>);

impl<M: MoneyHost> Default for MoneyMap<M> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

pub trait MoneyHost: AsMut<Money> {
    type Key: Hash + Eq + serde::Serialize + serde::de::DeserializeOwned;
    type Data: serde::Serialize + serde::de::DeserializeOwned;

    fn create(money: Money, data: Self::Data) -> Self;
}

impl<M: MoneyHost> MoneyMap<M> {
    pub(crate) fn create(
        &mut self,
        key: M::Key,
        currency_id: CurrencyId,
        data: M::Data,
    ) -> Result<(), MoneyError> {
        if self.0.contains_key(&key) {
            return Err(MoneyError::AlreadyExists);
        }

        self.0.insert(
            key,
            M::create(
                Money {
                    amount: Amount::default(),
                    currency_id,
                },
                data,
            ),
        );

        Ok(())
    }

    pub(crate) fn add(&mut self, key: M::Key, amount: Amount) -> Result<(), MoneyError> {
        let money = self.0.get_mut(&key).ok_or(MoneyError::NotFound)?;
        money.as_mut().amount += amount;

        Ok(())
    }

    pub(crate) fn sub(&mut self, key: M::Key, amount: Amount) -> Result<(), MoneyError> {
        let money = self.0.get_mut(&key).ok_or(MoneyError::NotFound)?;
        let stored_amount = &mut money.as_mut().amount;
        let result = stored_amount
            .checked_sub(amount)
            .ok_or(MoneyError::CannotSub)?;
        *stored_amount = result;

        Ok(())
    }

    pub(crate) fn remove(&mut self, key: M::Key) -> Result<(), MoneyError> {
        match self.0.remove(&key) {
            Some(_) => Ok(()),
            None => Err(MoneyError::NotFound),
        }
    }

    /// # Safety
    ///
    /// Creates a MoneyMap bypassing any domain rule.
    /// You must ensure that the data source is valid.
    /// Eg: Database, file save, etc.
    pub unsafe fn from_iter_unchecked<I>(iter: I) -> Self
    where
        I: Iterator<Item = (M::Key, M)>,
    {
        Self(iter.collect())
    }

    pub fn get(&self, k: &M::Key) -> Option<&M> {
        self.0.get(k)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, M::Key, M> {
        self.0.iter()
    }
}

impl<M: MoneyHost> IntoIterator for MoneyMap<M> {
    type Item = <HashMap<M::Key, M> as IntoIterator>::Item;
    type IntoIter = <HashMap<M::Key, M> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "money_error", rename_all = "snake_case")]
pub enum MoneyError {
    NotFound,
    CannotSub,
    AlreadyExists,
}
