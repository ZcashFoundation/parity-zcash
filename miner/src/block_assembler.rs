use chain::{
    IndexedTransaction, OutPoint, Transaction, TransactionInput, TransactionOutput,
    SAPLING_TX_VERSION, SAPLING_TX_VERSION_GROUP_ID,
};
use keys::Address;
use memory_pool::{Entry, MemoryPool, OrderingStrategy};
use network::ConsensusParams;
use primitives::compact::Compact;
use primitives::hash::H256;
use script::Builder;
use std::collections::HashSet;
use storage::{SaplingTreeState, SharedStore, TransactionOutputProvider};
use verification::{transaction_sigops, work_required};

const BLOCK_VERSION: u32 = 4;
const BLOCK_HEADER_SIZE: u32 = 4 + 32 + 32 + 32 + 4 + 4 + 32 + 1344;

/// Block template as described in [BIP0022](https://github.com/bitcoin/bips/blob/master/bip-0022.mediawiki#block-template-request)
pub struct BlockTemplate {
    /// Version
    pub version: u32,
    /// The hash of previous block
    pub previous_header_hash: H256,
    /// The hash of the final sapling root
    pub final_sapling_root_hash: H256,
    /// The current time as seen by the server
    pub time: u32,
    /// The compressed difficulty
    pub bits: Compact,
    /// Block height
    pub height: u32,
    /// Block transactions (excluding coinbase)
    pub transactions: Vec<IndexedTransaction>,
    /// Total funds available for the coinbase (in Satoshis)
    pub coinbase_tx: IndexedTransaction,
    /// Number of bytes allowed in the block
    pub size_limit: u32,
    /// Number of sigops allowed in the block
    pub sigop_limit: u32,
}

/// Block size and number of signatures opcodes is limited
/// This structure should be used for storing these values.
struct SizePolicy {
    /// Current size
    current_size: u32,
    /// Max size
    max_size: u32,
    /// When current_size + size_buffer > max_size
    /// we need to start finishing the block
    size_buffer: u32,
    /// Number of transactions checked since finishing started
    finish_counter: u32,
    /// Number of transactions to check when finishing the block
    finish_limit: u32,
}

/// When appending transaction, opcode count and block size policies
/// must agree on appending the transaction to the block
#[derive(Debug, PartialEq, Copy, Clone)]
enum NextStep {
    /// Append the transaction, check the next one
    Append,
    /// Append the transaction, do not check the next one
    FinishAndAppend,
    /// Ignore transaction, check the next one
    Ignore,
    /// Ignore transaction, do not check the next one
    FinishAndIgnore,
}

impl NextStep {
    fn and(self, other: NextStep) -> Self {
        match (self, other) {
            (_, NextStep::FinishAndIgnore)
            | (NextStep::FinishAndIgnore, _)
            | (NextStep::FinishAndAppend, NextStep::Ignore)
            | (NextStep::Ignore, NextStep::FinishAndAppend) => NextStep::FinishAndIgnore,

            (NextStep::Ignore, _) | (_, NextStep::Ignore) => NextStep::Ignore,

            (_, NextStep::FinishAndAppend) | (NextStep::FinishAndAppend, _) => {
                NextStep::FinishAndAppend
            }

            (NextStep::Append, NextStep::Append) => NextStep::Append,
        }
    }
}

impl SizePolicy {
    fn new(current_size: u32, max_size: u32, size_buffer: u32, finish_limit: u32) -> Self {
        SizePolicy {
            current_size: current_size,
            max_size: max_size,
            size_buffer: size_buffer,
            finish_counter: 0,
            finish_limit: finish_limit,
        }
    }

    fn decide(&mut self, size: u32) -> NextStep {
        let finishing = self.current_size + self.size_buffer > self.max_size;
        let fits = self.current_size + size <= self.max_size;
        let finish = self.finish_counter + 1 >= self.finish_limit;

        if finishing {
            self.finish_counter += 1;
        }

        match (fits, finish) {
            (true, true) => NextStep::FinishAndAppend,
            (true, false) => NextStep::Append,
            (false, true) => NextStep::FinishAndIgnore,
            (false, false) => NextStep::Ignore,
        }
    }

    fn apply(&mut self, size: u32) {
        self.current_size += size;
    }
}

/// Block assembler
pub struct BlockAssembler<'a> {
    /// Miner address.
    pub miner_address: &'a Address,
    /// Maximal block size.
    pub max_block_size: u32,
    /// Maximal # of sigops in the block.
    pub max_block_sigops: u32,
}

/// Iterator iterating over mempool transactions and yielding only those which fit the block
struct FittingTransactionsIterator<'a, T> {
    /// Shared store is used to query previous transaction outputs from database
    store: &'a TransactionOutputProvider,
    /// Memory pool transactions iterator
    iter: T,
    /// New block height
    block_height: u32,
    /// New block time
    block_time: u32,
    /// Size policy decides if transactions size fits the block
    block_size: SizePolicy,
    /// Sigops policy decides if transactions sigops fits the block
    sigops: SizePolicy,
    /// Previous entries are needed to get previous transaction outputs
    previous_entries: Vec<&'a Entry>,
    /// Hashes of ignored entries
    ignored: HashSet<H256>,
    /// True if block is already full
    finished: bool,
}

impl<'a, T> FittingTransactionsIterator<'a, T>
where
    T: Iterator<Item = &'a Entry>,
{
    fn new(
        store: &'a TransactionOutputProvider,
        iter: T,
        max_block_size: u32,
        max_block_sigops: u32,
        block_height: u32,
        block_time: u32,
    ) -> Self {
        FittingTransactionsIterator {
            store: store,
            iter: iter,
            block_height: block_height,
            block_time: block_time,
            // reserve some space for header and transactions len field
            block_size: SizePolicy::new(BLOCK_HEADER_SIZE + 4, max_block_size, 1_000, 50),
            sigops: SizePolicy::new(0, max_block_sigops, 8, 50),
            previous_entries: Vec::new(),
            ignored: HashSet::new(),
            finished: false,
        }
    }
}

impl<'a, T> TransactionOutputProvider for FittingTransactionsIterator<'a, T>
where
    T: Send + Sync,
{
    fn transaction_output(
        &self,
        prevout: &OutPoint,
        transaction_index: usize,
    ) -> Option<TransactionOutput> {
        self.store
            .transaction_output(prevout, transaction_index)
            .or_else(|| {
                self.previous_entries
                    .iter()
                    .find(|e| e.hash == prevout.hash)
                    .and_then(|e| e.transaction.outputs.iter().nth(prevout.index as usize))
                    .cloned()
            })
    }

    fn is_spent(&self, _outpoint: &OutPoint) -> bool {
        unimplemented!();
    }
}

impl<'a, T> Iterator for FittingTransactionsIterator<'a, T>
where
    T: Iterator<Item = &'a Entry> + Send + Sync,
{
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.finished {
            let entry = match self.iter.next() {
                Some(entry) => entry,
                None => {
                    self.finished = true;
                    return None;
                }
            };

            let transaction_size = entry.size as u32;
            let bip16_active = true;
            let sigops_count = transaction_sigops(&entry.transaction, self, bip16_active) as u32;

            let size_step = self.block_size.decide(transaction_size);
            let sigops_step = self.sigops.decide(sigops_count);

            // both next checks could be checked above, but then it will break finishing
            // check if transaction is still not finalized in this block
            if !entry
                .transaction
                .is_final_in_block(self.block_height, self.block_time)
            {
                continue;
            }
            // check if any parent transaction has been ignored
            if !self.ignored.is_empty()
                && entry
                    .transaction
                    .inputs
                    .iter()
                    .any(|input| self.ignored.contains(&input.previous_output.hash))
            {
                continue;
            }

            match size_step.and(sigops_step) {
                NextStep::Append => {
                    self.block_size.apply(transaction_size);
                    self.sigops.apply(transaction_size);
                    self.previous_entries.push(entry);
                    return Some(entry);
                }
                NextStep::FinishAndAppend => {
                    self.finished = true;
                    self.block_size.apply(transaction_size);
                    self.sigops.apply(transaction_size);
                    self.previous_entries.push(entry);
                    return Some(entry);
                }
                NextStep::Ignore => (),
                NextStep::FinishAndIgnore => {
                    self.ignored.insert(entry.hash.clone());
                    self.finished = true;
                }
            }
        }

        None
    }
}

impl<'a> BlockAssembler<'a> {
    pub fn create_new_block(
        &self,
        store: &SharedStore,
        mempool: &MemoryPool,
        time: u32,
        consensus: &ConsensusParams,
    ) -> Result<BlockTemplate, String> {
        // get best block
        // take its hash && height
        let best_block = store.best_block();
        let previous_header_hash = best_block.hash;
        let height = best_block.number + 1;
        let bits = work_required(
            previous_header_hash.clone(),
            time,
            height,
            store.as_block_header_provider(),
            consensus,
        );
        let version = BLOCK_VERSION;

        let mut miner_reward = consensus.miner_reward(height);
        let mut transactions = Vec::new();

        let mempool_iter = mempool.iter(OrderingStrategy::ByTransactionScore);
        let mut sapling_tree = if previous_header_hash.is_zero() {
            SaplingTreeState::new()
        } else {
            store
                .as_tree_state_provider()
                .sapling_tree_at_block(&previous_header_hash)
                .ok_or_else(|| {
                    format!(
                        "Sapling commitment tree for block {} is not found",
                        previous_header_hash.reversed()
                    )
                })?
        };
        let tx_iter = FittingTransactionsIterator::new(
            store.as_transaction_output_provider(),
            mempool_iter,
            self.max_block_size,
            self.max_block_sigops,
            height,
            time,
        );
        for entry in tx_iter {
            // miner_fee is i64, but we can safely cast it to u64
            // memory pool should restrict miner fee to be positive
            miner_reward += entry.miner_fee as u64;
            let tx = IndexedTransaction::new(entry.hash.clone(), entry.transaction.clone());
            if let Some(ref sapling) = tx.raw.sapling {
                for out in &sapling.outputs {
                    sapling_tree.append(out.note_commitment.into()).expect(
                        "only returns Err if tree is already full;
							sapling tree has height = 32;
							it means that there must be 2^32-1 sapling output descriptions to make it full;
							this should be impossible by consensus rules (i.e. it'll overflow block size before);
							qed",
                    );
                }
            }
            transactions.push(tx);
        }

        // prepare coinbase transaction
        let mut coinbase_tx = Transaction {
            overwintered: true,
            version: SAPLING_TX_VERSION,
            version_group_id: SAPLING_TX_VERSION_GROUP_ID,
            inputs: vec![TransactionInput::coinbase(
                Builder::default()
                    .push_i64(height.into())
                    .into_script()
                    .into(),
            )],
            outputs: vec![TransactionOutput {
                value: miner_reward,
                script_pubkey: Builder::build_p2pkh(&self.miner_address.hash).into(),
            }],
            lock_time: 0,
            expiry_height: 0,
            join_split: None,
            sapling: None,
        };

        // insert founder reward if required
        if let Some(founder_address) = consensus.founder_address(height) {
            coinbase_tx.outputs.push(TransactionOutput {
                value: consensus.founder_reward(height),
                script_pubkey: Builder::build_p2sh(&founder_address.hash).into(),
            });
        }

        Ok(BlockTemplate {
            version: version,
            previous_header_hash: previous_header_hash,
            final_sapling_root_hash: sapling_tree.root(),
            time: time,
            bits: bits,
            height: height,
            transactions: transactions,
            coinbase_tx: IndexedTransaction::from_raw(coinbase_tx),
            size_limit: self.max_block_size,
            sigop_limit: self.max_block_sigops,
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate test_data;

    use self::test_data::{ChainBuilder, TransactionBuilder};
    use super::{BlockAssembler, BlockTemplate, NextStep, SizePolicy};
    use chain::IndexedTransaction;
    use db::BlockChainDatabase;
    use fee::{FeeCalculator, NonZeroFeeCalculator};
    use memory_pool::MemoryPool;
    use network::{ConsensusParams, Network};
    use primitives::hash::H256;
    use std::sync::Arc;
    use storage::SharedStore;

    #[test]
    fn test_size_policy() {
        let mut size_policy = SizePolicy::new(0, 1000, 200, 3);
        assert_eq!(size_policy.decide(100), NextStep::Append);
        size_policy.apply(100);
        assert_eq!(size_policy.decide(500), NextStep::Append);
        size_policy.apply(500);
        assert_eq!(size_policy.decide(600), NextStep::Ignore);
        assert_eq!(size_policy.decide(200), NextStep::Append);
        size_policy.apply(200);
        assert_eq!(size_policy.decide(300), NextStep::Ignore);
        assert_eq!(size_policy.decide(300), NextStep::Ignore);
        // this transaction will make counter + buffer > max size
        assert_eq!(size_policy.decide(1), NextStep::Append);
        size_policy.apply(1);
        // so now only 3 more transactions may accepted / ignored
        assert_eq!(size_policy.decide(1), NextStep::Append);
        size_policy.apply(1);
        assert_eq!(size_policy.decide(1000), NextStep::Ignore);
        assert_eq!(size_policy.decide(1), NextStep::FinishAndAppend);
        size_policy.apply(1);
        // we should not call decide again after it returned finish...
        // but we can, let's check if result is ok
        assert_eq!(size_policy.decide(1000), NextStep::FinishAndIgnore);
    }

    #[test]
    fn test_next_step_and() {
        assert_eq!(NextStep::Append.and(NextStep::Append), NextStep::Append);
        assert_eq!(NextStep::Ignore.and(NextStep::Append), NextStep::Ignore);
        assert_eq!(
            NextStep::FinishAndIgnore.and(NextStep::Append),
            NextStep::FinishAndIgnore
        );
        assert_eq!(
            NextStep::Ignore.and(NextStep::FinishAndIgnore),
            NextStep::FinishAndIgnore
        );
        assert_eq!(
            NextStep::FinishAndAppend.and(NextStep::FinishAndIgnore),
            NextStep::FinishAndIgnore
        );
        assert_eq!(
            NextStep::FinishAndAppend.and(NextStep::Ignore),
            NextStep::FinishAndIgnore
        );
        assert_eq!(
            NextStep::FinishAndAppend.and(NextStep::Append),
            NextStep::FinishAndAppend
        );
    }

    #[test]
    fn test_fitting_transactions_iterator_max_block_size_reached() {}

    #[test]
    fn test_fitting_transactions_iterator_ignored_parent() {
        // TODO
    }

    #[test]
    fn test_fitting_transactions_iterator_locked_transaction() {
        // TODO
    }

    #[test]
    fn block_assembler_transaction_order() {
        fn construct_block(consensus: ConsensusParams) -> (BlockTemplate, H256, H256) {
            let chain = &mut ChainBuilder::new();
            TransactionBuilder::with_default_input(0)
                .set_output(30)
                .store(chain) // transaction0
                .into_input(0)
                .set_output(50)
                .store(chain); // transaction0 -> transaction1
            let hash0 = chain.at(0).hash();
            let hash1 = chain.at(1).hash();

            let mut pool = MemoryPool::new();
            let storage: SharedStore = Arc::new(BlockChainDatabase::init_test_chain(vec![
                test_data::genesis().into(),
            ]));
            pool.insert_verified(chain.at(0).into(), &NonZeroFeeCalculator);
            pool.insert_verified(chain.at(1).into(), &NonZeroFeeCalculator);

            (
                BlockAssembler {
                    miner_address: &"t1h8SqgtM3QM5e2M8EzhhT1yL2PXXtA6oqe".into(),
                    max_block_size: 0xffffffff,
                    max_block_sigops: 0xffffffff,
                }
                .create_new_block(&storage, &pool, 0, &consensus)
                .unwrap(),
                hash0,
                hash1,
            )
        }

        // when topological consensus is used
        let topological_consensus = ConsensusParams::new(Network::Mainnet);
        let (block, hash0, hash1) = construct_block(topological_consensus);
        assert!(hash1 < hash0);
        assert_eq!(block.transactions[0].hash, hash0);
        assert_eq!(block.transactions[1].hash, hash1);
    }

    #[test]
    fn block_assembler_miner_fee() {
        let input_tx = test_data::block_h1().transactions[0].clone();
        let tx0: IndexedTransaction = TransactionBuilder::with_input(&input_tx, 0)
            .set_output(10_000)
            .into();
        let expected_tx0_fee = input_tx.outputs[0].value - tx0.raw.total_spends();

        let storage: SharedStore = Arc::new(BlockChainDatabase::init_test_chain(vec![
            test_data::genesis().into(),
            test_data::block_h1().into(),
        ]));
        let mut pool = MemoryPool::new();
        pool.insert_verified(
            tx0,
            &FeeCalculator(storage.as_transaction_output_provider()),
        );

        let consensus = ConsensusParams::new(Network::Mainnet);
        let block = BlockAssembler {
            max_block_size: 0xffffffff,
            max_block_sigops: 0xffffffff,
            miner_address: &"t1h8SqgtM3QM5e2M8EzhhT1yL2PXXtA6oqe".into(),
        }
        .create_new_block(&storage, &pool, 0, &consensus)
        .unwrap();

        let expected_coinbase_value = consensus.block_reward(2) + expected_tx0_fee;
        assert_eq!(
            block.coinbase_tx.raw.total_spends(),
            expected_coinbase_value
        );
    }
}
