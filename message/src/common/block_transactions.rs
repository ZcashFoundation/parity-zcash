use hash::H256;
use zebra_chain::Transaction;

#[derive(Debug, PartialEq, Serializable, Deserializable)]
pub struct BlockTransactions {
    pub blockhash: H256,
    pub transactions: Vec<Transaction>,
}
