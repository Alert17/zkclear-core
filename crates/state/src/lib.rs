use std::collections::HashMap;
use zkclear_types::{Account, AccountId, Address, Deal, DealId};

#[derive(Debug, Default)]
pub struct State {
    pub accounts: HashMap<AccountId, Account>,
    pub deals: HashMap<DealId, Deal>,
    pub account_index: HashMap<Address, AccountId>,
    pub next_account_id: AccountId,
}

impl State {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            deals: HashMap::new(),
            account_index: HashMap::new(),
            next_account_id: 0,
        }
    }

    pub fn get_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(&id)
    }

    pub fn get_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(&id)
    }

    pub fn upsert_account(&mut self, account: Account) {
        self.account_index.insert(account.owner, account.id);
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

    pub fn get_or_create_account_by_owner(&mut self, owner: Address) -> &mut Account {
        if let Some(id) = self.account_index.get(&owner).cloned() {
            return self.accounts.get_mut(&id).expect("inconsistent state");
        }

        let id = self.next_account_id;
        self.next_account_id = self.next_account_id.wrapping_add(1);

        let account = Account {
            id,
            owner,
            balances: Vec::new(),
            nonce: 0,
            created_at: 0,
        };

        self.accounts.insert(id, account);
        self.account_index.insert(owner, id);
        self.accounts.get_mut(&id).expect("just inserted")
    }
}
