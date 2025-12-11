use zkclear_state::State;
use zkclear_types::{
    Account,
    AssetId,
    Address,
    Balance,
    Deposit,
    Withdraw,
    CreateDeal,
    AcceptDeal,
    CancelDeal,
    Deal,
    DealStatus,
    DealVisibility,
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
}

pub fn apply_block(state: &mut State, txs: &[Tx]) -> Result<(), StfError> {
    for tx in txs {
        apply_tx(state, tx)?;
    }
    Ok(())
}

pub fn apply_tx(state: &mut State, tx: &Tx) -> Result<(), StfError> {
    match &tx.payload {
        TxPayload::Deposit(p) => apply_deposit(state, p),
        TxPayload::Withdraw(p) => apply_withdraw(state, p),
        TxPayload::CreateDeal(p) => apply_create_deal(state, tx.from, p),
        TxPayload::AcceptDeal(p) => apply_accept_deal(state, tx.from, p),
        TxPayload::CancelDeal(p) => apply_cancel_deal(state, tx.from, p),
    }
}

fn apply_deposit(state: &mut State, payload: &Deposit) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(payload.account);
    add_balance(account, payload.asset_id, payload.amount);
    Ok(())
}

fn apply_withdraw(state: &mut State, payload: &Withdraw) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(payload.account);
    sub_balance(account, payload.asset_id, payload.amount)?;
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
    let deal = state
        .get_deal_mut(payload.deal_id)
        .ok_or(StfError::DealNotFound)?;

    if deal.status != DealStatus::Pending {
        return Err(StfError::DealAlreadyClosed);
    }

    match deal.visibility {
        DealVisibility::Public => {}
        DealVisibility::Direct => {
            if let Some(expected_taker) = deal.taker {
                if expected_taker != taker {
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

    let amount_base = deal.amount_base;
    let amount_quote = amount_base
        .checked_mul(deal.price_quote_per_base)
        .ok_or(StfError::Overflow)?;

    let maker_addr = deal.maker;

    let maker_account = state.get_or_create_account_by_owner(maker_addr);
    let taker_account = state.get_or_create_account_by_owner(taker);

    sub_balance(maker_account, deal.asset_base, amount_base)?;
    sub_balance(taker_account, deal.asset_quote, amount_quote)?;

    add_balance(maker_account, deal.asset_quote, amount_quote);
    add_balance(taker_account, deal.asset_base, amount_base);

    deal.status = DealStatus::Settled;

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

fn add_balance(account: &mut Account, asset_id: AssetId, amount: u128) {
    for b in &mut account.balances {
        if b.asset_id == asset_id {
            b.amount = b.amount.saturating_add(amount);
            return;
        }
    }

    account.balances.push(Balance { asset_id, amount });
}

fn sub_balance(account: &mut Account, asset_id: AssetId, amount: u128) -> Result<(), StfError> {
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
