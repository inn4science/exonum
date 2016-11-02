use serde::{Serialize, Serializer};

use exonum::crypto::{HexValue, PublicKey};
use exonum::storage::{Database, Result as StorageResult};
use exonum::blockchain::Blockchain;
use exonum::node::Configuration;
use blockchain_explorer::{BlockchainExplorer, TransactionInfo};

use super::{CurrencyTx, CurrencyBlockchain};
use super::wallet::{Wallet, WalletId};

impl Serialize for CurrencyTx {
    fn serialize<S>(&self, ser: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        let mut state;
        match *self {
            CurrencyTx::Issue(ref issue) => {
                state = ser.serialize_struct("transaction", 4)?;
                ser.serialize_struct_elt(&mut state, "type", "issue")?;
                ser.serialize_struct_elt(&mut state, "wallet", issue.wallet().to_hex())?;
                ser.serialize_struct_elt(&mut state, "amount", issue.amount())?;
                ser.serialize_struct_elt(&mut state, "seed", issue.seed())?;
            }
            CurrencyTx::Transfer(ref transfer) => {
                state = ser.serialize_struct("transaction", 5)?;
                ser.serialize_struct_elt(&mut state, "type", "transfer")?;
                ser.serialize_struct_elt(&mut state, "from", transfer.from().to_hex())?;
                ser.serialize_struct_elt(&mut state, "to", transfer.to().to_hex())?;
                ser.serialize_struct_elt(&mut state, "amount", transfer.amount())?;
                ser.serialize_struct_elt(&mut state, "seed", transfer.seed())?;
            }
            CurrencyTx::CreateWallet(ref wallet) => {
                state = ser.serialize_struct("transaction", 3)?;
                ser.serialize_struct_elt(&mut state, "type", "create_wallet")?;
                ser.serialize_struct_elt(&mut state, "pub_key", wallet.pub_key().to_hex())?;
                ser.serialize_struct_elt(&mut state, "name", wallet.name())?;
            }
        }
        ser.serialize_struct_end(state)
    }
}

impl TransactionInfo for CurrencyTx {}

pub struct WalletInfo {
    inner: Wallet,
    id: WalletId,
    history: Vec<CurrencyTx>,
}

impl Serialize for WalletInfo {
    fn serialize<S>(&self, ser: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        let mut state = ser.serialize_struct("wallet", 7)?;
        ser.serialize_struct_elt(&mut state, "id", self.id)?;
        ser.serialize_struct_elt(&mut state, "balance", self.inner.balance())?;
        ser.serialize_struct_elt(&mut state, "name", self.inner.name())?;
        ser.serialize_struct_elt(&mut state, "history", &self.history)?;
        ser.serialize_struct_elt(&mut state,
                                  "history_hash",
                                  self.inner.history_hash().to_hex())?;
        ser.serialize_struct_end(state)
    }
}

pub struct CurrencyApi<D: Database> {
    blockchain: CurrencyBlockchain<D>,
    cfg: Configuration,
}

impl<D: Database> CurrencyApi<D> {
    pub fn new(b: CurrencyBlockchain<D>, cfg: Configuration) -> CurrencyApi<D> {
        CurrencyApi {
            blockchain: b,
            cfg: cfg,
        }
    }

    pub fn wallet_info(&self, pub_key: &PublicKey) -> StorageResult<Option<WalletInfo>> {
        let view = self.blockchain.view();
        if let Some((id, wallet)) = view.wallet(pub_key)? {
            let history = view.wallet_history(id).values()?;
            let txs = {
                let mut v = Vec::new();

                let explorer =
                    BlockchainExplorer::<CurrencyBlockchain<D>>::from_view(view, self.cfg.clone());
                for hash in history {
                    if let Some(tx_info) = explorer.tx_info::<CurrencyTx>(&hash)? {
                        v.push(tx_info)
                    }
                }
                v
            };

            let info = WalletInfo {
                id: id,
                inner: wallet,
                history: txs,
            };
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}