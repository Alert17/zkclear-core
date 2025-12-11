use zkclear_state::State;
use zkclear_types::{
    AssetId,
    Balance,
    Deposit,
    Withdraw,
    CreateDeal,
    AcceptDeal,
    CancelDeal,
    Deal,
    DealStatus,
    DealVisibility,
    Address,
    Tx,
    TxPayload,
};

#[derive(Debug)]
pub enum StfError {
    UnsupportedTx,
    NotImplemented,
    BalanceTooLow,
    DealNotFound,
    DealAlreadyClosed,
    DealAlreadyExists,
    Unauthorized,
    Overflow,
    InvalidNonce,
    DealExpired,
}

pub fn apply_tx(state: &mut State, tx: &Tx) -> Result<(), StfError> {
    validate_nonce(state, tx.from, tx.nonce)?;
    
    let result = match &tx.payload {
        TxPayload::Deposit(p)    => apply_deposit(state, p),
        TxPayload::Withdraw(p)   => apply_withdraw(state, tx.from, p),
        TxPayload::CreateDeal(p) => apply_create_deal(state, tx.from, p),
        TxPayload::AcceptDeal(p) => apply_accept_deal(state, tx.from, p),
        TxPayload::CancelDeal(p) => apply_cancel_deal(state, tx.from, p),
    };
    
    if result.is_ok() {
        increment_nonce(state, tx.from);
    }
    
    result
}

fn apply_deposit(state: &mut State, payload: &Deposit) -> Result<(), StfError> {
    add_balance(state, payload.account, payload.asset_id, payload.amount);
    Ok(())
}

fn apply_withdraw(state: &mut State, from: Address, payload: &Withdraw) -> Result<(), StfError> {
    sub_balance(state, from, payload.asset_id, payload.amount)
}

pub fn apply_block(state: &mut State, txs: &[Tx]) -> Result<(), StfError> {
    for tx in txs {
        apply_tx(state, tx)?;
    }
    Ok(())
}

fn apply_create_deal(state: &mut State, maker: Address, payload: &CreateDeal) -> Result<(), StfError> {
    if state.get_deal(payload.deal_id).is_some() {
        return Err(StfError::DealAlreadyExists);
    }

    let deal = Deal {
        id: payload.deal_id,
        maker,
        taker: payload.taker,
        visibility: payload.visibility,
        asset_base: payload.asset_base,
        asset_quote: payload.asset_quote,
        amount_base: payload.amount_base,
        amount_remaining: payload.amount_base,
        price_quote_per_base: payload.price_quote_per_base,
        status: DealStatus::Pending,
        created_at: 0,
        expires_at: payload.expires_at,
        external_ref: payload.external_ref.clone(),
    };

    state.upsert_deal(deal);

    Ok(())
}

fn apply_accept_deal(state: &mut State, taker: Address, payload: &AcceptDeal) -> Result<(), StfError> {
    let (maker_addr, asset_base, asset_quote, amount_remaining, price_quote_per_base, _expires_at, _visibility, _expected_taker) = {
        let deal = state
            .get_deal(payload.deal_id)
            .ok_or(StfError::DealNotFound)?;

        if deal.status != DealStatus::Pending {
            return Err(StfError::DealAlreadyClosed);
        }

        if let Some(exp) = deal.expires_at {
            if exp > 0 && exp < get_current_timestamp() {
                return Err(StfError::DealExpired);
            }
        }

        match deal.visibility {
            DealVisibility::Public => {}
            DealVisibility::Direct => {
                if let Some(expected) = deal.taker {
                    if expected != taker {
                        return Err(StfError::Unauthorized);
                    }
                } else {
                    return Err(StfError::Unauthorized);
                }
            }
        }

        if deal.maker == taker {
            return Err(StfError::Unauthorized);
        }

        (
            deal.maker,
            deal.asset_base,
            deal.asset_quote,
            deal.amount_remaining,
            deal.price_quote_per_base,
            deal.expires_at,
            deal.visibility,
            deal.taker,
        )
    };

    let amount_to_fill = payload.amount.unwrap_or(amount_remaining);
    if amount_to_fill == 0 || amount_to_fill > amount_remaining {
        return Err(StfError::BalanceTooLow);
    }

    let amount_quote = amount_to_fill
        .checked_mul(price_quote_per_base)
        .ok_or(StfError::Overflow)?;

    ensure_balance(state, maker_addr, asset_base, amount_to_fill)?;
    ensure_balance(state, taker, asset_quote, amount_quote)?;

    sub_balance(state, maker_addr, asset_base, amount_to_fill)?;
    sub_balance(state, taker, asset_quote, amount_quote)?;

    add_balance(state, maker_addr, asset_quote, amount_quote);
    add_balance(state, taker, asset_base, amount_to_fill);

    let deal = state
        .get_deal_mut(payload.deal_id)
        .ok_or(StfError::DealNotFound)?;
    deal.amount_remaining -= amount_to_fill;
    if deal.amount_remaining == 0 {
        deal.status = DealStatus::Settled;
    }

    Ok(())
}

fn apply_cancel_deal(state: &mut State, caller: Address, payload: &CancelDeal) -> Result<(), StfError> {
    let deal = state
        .get_deal_mut(payload.deal_id)
        .ok_or(StfError::DealNotFound)?;

    if deal.status != DealStatus::Pending {
        return Err(StfError::DealAlreadyClosed);
    }

    if deal.maker != caller {
        return Err(StfError::Unauthorized);
    }

    deal.status = DealStatus::Cancelled;

    Ok(())
}

fn add_balance(state: &mut State, owner: Address, asset_id: AssetId, amount: u128) {
    let account = state.get_or_create_account_by_owner(owner);

    for b in &mut account.balances {
        if b.asset_id == asset_id {
            b.amount = b.amount.saturating_add(amount);
            return;
        }
    }

    account.balances.push(Balance {
        asset_id,
        amount,
    });
}

fn sub_balance(state: &mut State, owner: Address, asset_id: AssetId, amount: u128) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(owner);

    for b in &mut account.balances {
        if b.asset_id == asset_id {
            if b.amount < amount {
                return Err(StfError::BalanceTooLow);
            }
            b.amount -= amount;
            return Ok(());
        }
    }

    Err(StfError::BalanceTooLow)
}

fn ensure_balance(state: &mut State, owner: Address, asset_id: AssetId, amount: u128) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(owner);

    for b in &account.balances {
        if b.asset_id == asset_id {
            if b.amount < amount {
                return Err(StfError::BalanceTooLow);
            }
            return Ok(());
        }
    }

    Err(StfError::BalanceTooLow)
}

fn validate_nonce(state: &mut State, owner: Address, tx_nonce: u64) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(owner);
    let expected_nonce = account.nonce;
    
    if tx_nonce != expected_nonce {
        return Err(StfError::InvalidNonce);
    }
    
    Ok(())
}

fn increment_nonce(state: &mut State, owner: Address) {
    let account = state.get_or_create_account_by_owner(owner);
    account.nonce += 1;
}

fn get_current_timestamp() -> u64 {
    0
}
