pub type AccountId = u64;
pub type DealId = u64;
pub type AssetId = u16;

pub type Address = [u8; 20];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DealVisibility {
    Public,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DealStatus {
    Pending,
    Settled,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: AccountId,
    pub owner: Address,
    pub balances: Vec<Balance>,
    pub nonce: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub asset_id: AssetId,
    pub amount: u128,
}

#[derive(Debug, Clone)]
pub struct Deal {
    pub id: DealId,
    pub maker: Address,
    pub taker: Option<Address>,
    pub visibility: DealVisibility,
    pub asset_base: AssetId,
    pub asset_quote: AssetId,
    pub amount_base: u128,
    pub price_quote_per_base: u128,
    pub status: DealStatus,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TxKind {
    Deposit,
    CreateDeal,
    AcceptDeal,
    CancelDeal,
    Withdraw,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub id: u64,
    pub from: Address,
    pub nonce: u64,
    pub kind: TxKind,
    pub payload: TxPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum TxPayload {
    Deposit(Deposit),
    CreateDeal(CreateDeal),
    AcceptDeal(AcceptDeal),
    CancelDeal(CancelDeal),
    Withdraw(Withdraw),
}

#[derive(Debug, Clone)]
pub struct Deposit {
    pub tx_hash: [u8; 32],
    pub account: Address,
    pub asset_id: AssetId,
    pub amount: u128,
}

#[derive(Debug, Clone)]
pub struct CreateDeal {
    pub deal_id: DealId,
    pub visibility: DealVisibility,
    pub taker: Option<Address>,
    pub asset_base: AssetId,
    pub asset_quote: AssetId,
    pub amount_base: u128,
    pub price_quote_per_base: u128,
    pub expires_at: Option<u64>,
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AcceptDeal {
    pub deal_id: DealId,
}

#[derive(Debug, Clone)]
pub struct CancelDeal {
    pub deal_id: DealId,
}

#[derive(Debug, Clone)]
pub struct Withdraw {
    pub asset_id: AssetId,
    pub amount: u128,
    pub to: Address,
}
