use std::collections::HashMap;
use zkclear_types::{Account, AccountId, Deal, DealId};

#[derive(Debug, Default)]
pub struct State {
    pub accounts: HashMap<AccountId, Account>,
    pub deals: HashMap<DealId, Deal>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(&id)
    }

    pub fn get_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(&id)
    }

    pub fn upsert_account(&mut self, account: Account) {
        self.accounts.insert(account.id, account);
    }

    pub fn get_deal(&self, id: DealId) -> Option<&Deal> {
        self.deals.get(&id)
    }

    pub fn get_deal_mut(&mut self, id: DealId) -> Option<&mut Deal> {
        self.deals.get_mut(&id)
    }

    pub fn upsert_deal(&mut self, deal: Deal) {
        self.deals.insert(deal.id, deal);
    }
}
