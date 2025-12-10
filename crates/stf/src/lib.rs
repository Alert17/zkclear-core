use zkclear_state::State;
use zkclear_types::{
    Balance,
    Deposit,
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
    Unauthorized,
}


pub fn apply_tx(state: &mut State, tx: &Tx) -> Result<(), StfError> {
    match &tx.payload {
        TxPayload::Deposit(p)      => apply_deposit(state, p),
        TxPayload::Withdraw(p)     => apply_withdraw(state, p),
        TxPayload::CreateDeal(p)   => apply_create_deal(state, p),
        TxPayload::AcceptDeal(p)   => apply_accept_deal(state, p),
        TxPayload::CancelDeal(p)   => apply_cancel_deal(state, p),
    }
}

fn apply_deposit_(state: &mut State, payload: &Deposit) -> Result<(), StfError> {
    let account = state.get_or_create_account_by_owner(payload.account);

    let mut found = false;
    for b in &mut account.balances {
        if b.asset_id == payload.asset_id {
            b.amount = b.amount.saturating_add(payload.amount);
            found = true;
            break;
        }
    }

    if !found {
        account.balances.push(Balance {
            asset_id: payload.asset_id,
            amount: payload.amount,
        });
    }

    Ok(())
}

pub fn apply_block(state: &mut State, txs: &[Tx]) -> Result<(), StfError> {
    for tx in txs {
        apply_tx(state, tx)?;
    }
    Ok(())
}

fn apply_create_deal(state: &mut State, payload: &CreateDeal) -> Result<(), StfError> {
    Err(StfError::NotImplemented)
}

fn apply_accept_deal(state: &mut State, payload: &AcceptDeal) -> Result<(), StfError> {
    Err(StfError::NotImplemented)
}

fn apply_cancel_deal(state: &mut State, payload: &CancelDeal) -> Result<(), StfError> {
    Err(StfError::NotImplemented)
}
