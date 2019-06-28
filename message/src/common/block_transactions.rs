use chain::Transaction;
use hash::H256;

#[derive(Debug, PartialEq, Serializable, Deserializable)]
pub struct BlockTransactions {
    pub blockhash: H256,
    pub transactions: Vec<Transaction>,
}
