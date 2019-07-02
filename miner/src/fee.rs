use ser::Serializable;
use zebra_chain::Transaction;
use zebra_storage::{DuplexTransactionOutputProvider, TransactionOutputProvider};
use zebra_verification::checked_transaction_fee;
use MemoryPool;

/// Transaction fee calculator for memory pool
pub trait MemoryPoolFeeCalculator {
    /// Compute transaction fee
    fn calculate(&self, memory_pool: &MemoryPool, tx: &Transaction) -> u64;
}

/// Fee calculator that computes sum of real transparent fee + real shielded fee.
pub struct FeeCalculator<'a>(pub &'a TransactionOutputProvider);

impl<'a> MemoryPoolFeeCalculator for FeeCalculator<'a> {
    fn calculate(&self, memory_pool: &MemoryPool, tx: &Transaction) -> u64 {
        let tx_out_provider = DuplexTransactionOutputProvider::new(self.0, memory_pool);
        transaction_fee(&tx_out_provider, tx)
    }
}

/// Used in tests in this && external crates
#[cfg(any(test, feature = "test-helpers"))]
pub struct NonZeroFeeCalculator;

#[cfg(any(test, feature = "test-helpers"))]
impl MemoryPoolFeeCalculator for NonZeroFeeCalculator {
    fn calculate(&self, _: &MemoryPool, tx: &Transaction) -> u64 {
        // add 100_000_000 to make sure tx won't be rejected by txpoool because of fee
        // + but keep ordering by outputs sum
        100_000_000 + tx.outputs.iter().fold(0, |acc, output| acc + output.value)
    }
}

/// Compute miner fee for given (memory pool) transaction.
///
/// If any error occurs during computation, zero fee is returned. Normally, zero fee
/// transactions are not accepted to the memory pool.
pub fn transaction_fee(store: &TransactionOutputProvider, tx: &Transaction) -> u64 {
    checked_transaction_fee(store, ::std::usize::MAX, tx).unwrap_or(0)
}

pub fn transaction_fee_rate(store: &TransactionOutputProvider, tx: &Transaction) -> u64 {
    transaction_fee(store, tx) / tx.serialized_size() as u64
}

#[cfg(test)]
mod tests {
    use super::transaction_fee_rate;
    use std::sync::Arc;
    use zebra_db::BlockChainDatabase;
    use zebra_storage::AsSubstore;

    #[test]
    fn transaction_fee_rate_works() {
        let b0 = zebra_test_data::block_builder()
            .header()
            .nonce(1.into())
            .build()
            .transaction()
            .output()
            .value(1_000_000)
            .build()
            .output()
            .value(2_000_000)
            .build()
            .build()
            .build();
        let tx0 = b0.transactions[0].clone();
        let tx0_hash = tx0.hash();
        let b1 = zebra_test_data::block_builder()
            .header()
            .parent(b0.hash().clone())
            .nonce(2.into())
            .build()
            .transaction()
            .input()
            .hash(tx0_hash.clone())
            .index(0)
            .build()
            .input()
            .hash(tx0_hash)
            .index(1)
            .build()
            .output()
            .value(2_500_000)
            .build()
            .build()
            .build();
        let tx2 = b1.transactions[0].clone();

        let db = Arc::new(BlockChainDatabase::init_test_chain(vec![
            b0.into(),
            b1.into(),
        ]));
        let store = db.as_transaction_output_provider();

        assert_eq!(transaction_fee_rate(store, &tx0), 0);
        assert_eq!(transaction_fee_rate(store, &tx2), 4_901);
    }
}
