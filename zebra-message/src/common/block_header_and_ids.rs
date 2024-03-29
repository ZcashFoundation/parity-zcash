use common::PrefilledTransaction;
use zebra_chain::{BlockHeader, ShortTransactionID};

#[derive(Debug, PartialEq, Serializable, Deserializable)]
pub struct BlockHeaderAndIDs {
    pub header: BlockHeader,
    pub nonce: u64,
    pub short_ids: Vec<ShortTransactionID>,
    pub prefilled_transactions: Vec<PrefilledTransaction>,
}
