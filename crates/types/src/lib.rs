pub type AccountId = u64;
pub type DealId = u64;
pub type AssetId = u16;
pub type BlockId = u64;

pub type Address = [u8; 20];
pub type Signature = [u8; 65];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DealVisibility {
    Public,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DealStatus {
    Pending,
    Settled,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub id: AccountId,
    #[serde(with = "serde_bytes")]
    pub owner: Address,
    pub balances: Vec<Balance>,
    pub nonce: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    pub asset_id: AssetId,
    pub amount: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Deal {
    pub id: DealId,
    pub maker: Address,
    pub taker: Option<Address>,
    pub visibility: DealVisibility,
    pub asset_base: AssetId,
    pub asset_quote: AssetId,
    pub amount_base: u128,
    pub amount_remaining: u128,
    pub price_quote_per_base: u128,
    pub status: DealStatus,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TxKind {
    Deposit,
    CreateDeal,
    AcceptDeal,
    CancelDeal,
    Withdraw,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tx {
    pub id: u64,
    #[serde(with = "serde_bytes")]
    pub from: Address,
    pub nonce: u64,
    pub kind: TxKind,
    pub payload: TxPayload,
    #[serde(with = "serde_bytes")]
    pub signature: Signature,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TxPayload {
    Deposit(Deposit),
    CreateDeal(CreateDeal),
    AcceptDeal(AcceptDeal),
    CancelDeal(CancelDeal),
    Withdraw(Withdraw),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Deposit {
    #[serde(with = "serde_bytes")]
    pub tx_hash: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub account: Address,
    pub asset_id: AssetId,
    pub amount: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcceptDeal {
    pub deal_id: DealId,
    pub amount: Option<u128>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CancelDeal {
    pub deal_id: DealId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Withdraw {
    pub asset_id: AssetId,
    pub amount: u128,
    pub to: Address,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub transactions: Vec<Tx>,
    pub timestamp: u64,
}
