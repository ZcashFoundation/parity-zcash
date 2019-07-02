//! Bitcoin chain verifier

use accept_chain::ChainAcceptor;
use accept_transaction::MemoryPoolTransactionAcceptor;
use canon::{CanonBlock, CanonTransaction};
use deployments::{BlockDeployments, Deployments};
use error::{Error, TransactionError};
use verify_chain::ChainVerifier;
use verify_header::HeaderVerifier;
use verify_transaction::MemoryPoolTransactionVerifier;
use zebra_chain::{IndexedBlock, IndexedBlockHeader, IndexedTransaction};
use zebra_network::ConsensusParams;
use zebra_storage::{
    BlockHeaderProvider, BlockOrigin, CachedTransactionOutputProvider,
    DuplexTransactionOutputProvider, NoopStore, SharedStore, TransactionOutputProvider,
};
use {VerificationLevel, Verify};

pub struct BackwardsCompatibleChainVerifier {
    store: SharedStore,
    consensus: ConsensusParams,
    deployments: Deployments,
}

impl BackwardsCompatibleChainVerifier {
    pub fn new(store: SharedStore, consensus: ConsensusParams) -> Self {
        BackwardsCompatibleChainVerifier {
            store: store,
            consensus: consensus,
            deployments: Deployments::new(),
        }
    }

    fn verify_block(
        &self,
        verification_level: VerificationLevel,
        block: &IndexedBlock,
    ) -> Result<(), Error> {
        if verification_level.intersects(VerificationLevel::NO_VERIFICATION) {
            return Ok(());
        }

        let current_time = ::time::get_time().sec as u32;
        // first run pre-verification
        let chain_verifier =
            ChainVerifier::new(block, &self.consensus, current_time, verification_level);
        chain_verifier.check()?;

        assert_eq!(
            Some(self.store.best_block().hash),
            self.store.block_hash(self.store.best_block().number)
        );
        let block_origin = self.store.block_origin(&block.header)?;
        trace!(
            target: "verification",
            "verify_block: {:?} best_block: {:?} block_origin: {:?}",
            block.hash().reversed(),
            self.store.best_block(),
            block_origin,
        );

        let canon_block = CanonBlock::new(block);
        match block_origin {
            BlockOrigin::KnownBlock => {
                // there should be no known blocks at this point
                unreachable!(
                    "Trying to re-verify known block: {}",
                    block.hash().reversed()
                );
            }
            BlockOrigin::CanonChain { block_number } => {
                let tx_out_provider = CachedTransactionOutputProvider::new(
                    self.store.as_store().as_transaction_output_provider(),
                );
                let tx_meta_provider = self.store.as_store().as_transaction_meta_provider();
                let header_provider = self.store.as_store().as_block_header_provider();
                let tree_state_provider = self.store.as_store().as_tree_state_provider();
                let nullifier_tracker = self.store.as_store().as_nullifier_tracker();
                let deployments = BlockDeployments::new(
                    &self.deployments,
                    block_number,
                    header_provider,
                    &self.consensus,
                );
                let chain_acceptor = ChainAcceptor::new(
                    &tx_out_provider,
                    tx_meta_provider,
                    header_provider,
                    tree_state_provider,
                    nullifier_tracker,
                    &self.consensus,
                    verification_level,
                    canon_block,
                    block_number,
                    block.header.raw.time,
                    &deployments,
                );
                chain_acceptor.check()?;
            }
            BlockOrigin::SideChain(origin) => {
                let block_number = origin.block_number;
                let fork = self.store.fork(origin)?;
                let tx_out_provider = CachedTransactionOutputProvider::new(
                    fork.store().as_transaction_output_provider(),
                );
                let tx_meta_provider = fork.store().as_transaction_meta_provider();
                let header_provider = fork.store().as_block_header_provider();
                let tree_state_provider = fork.store().as_tree_state_provider();
                let nullifier_tracker = fork.store().as_nullifier_tracker();
                let deployments = BlockDeployments::new(
                    &self.deployments,
                    block_number,
                    header_provider,
                    &self.consensus,
                );
                let chain_acceptor = ChainAcceptor::new(
                    &tx_out_provider,
                    tx_meta_provider,
                    header_provider,
                    tree_state_provider,
                    nullifier_tracker,
                    &self.consensus,
                    verification_level,
                    canon_block,
                    block_number,
                    block.header.raw.time,
                    &deployments,
                );
                chain_acceptor.check()?;
            }
            BlockOrigin::SideChainBecomingCanonChain(origin) => {
                let block_number = origin.block_number;
                let fork = self.store.fork(origin)?;
                let tx_out_provider = CachedTransactionOutputProvider::new(
                    fork.store().as_transaction_output_provider(),
                );
                let tx_meta_provider = fork.store().as_transaction_meta_provider();
                let header_provider = fork.store().as_block_header_provider();
                let tree_state_provider = fork.store().as_tree_state_provider();
                let nullifier_tracker = fork.store().as_nullifier_tracker();
                let deployments = BlockDeployments::new(
                    &self.deployments,
                    block_number,
                    header_provider,
                    &self.consensus,
                );
                let chain_acceptor = ChainAcceptor::new(
                    &tx_out_provider,
                    tx_meta_provider,
                    header_provider,
                    tree_state_provider,
                    nullifier_tracker,
                    &self.consensus,
                    verification_level,
                    canon_block,
                    block_number,
                    block.header.raw.time,
                    &deployments,
                );
                chain_acceptor.check()?;
            }
        };

        assert_eq!(
            Some(self.store.best_block().hash),
            self.store.block_hash(self.store.best_block().number)
        );
        Ok(())
    }

    pub fn verify_block_header(&self, header: &IndexedBlockHeader) -> Result<(), Error> {
        let current_time = ::time::get_time().sec as u32;
        let header_verifier = HeaderVerifier::new(header, &self.consensus, current_time);
        header_verifier.check()
    }

    pub fn verify_mempool_transaction<T>(
        &self,
        block_header_provider: &BlockHeaderProvider,
        prevout_provider: &T,
        height: u32,
        time: u32,
        transaction: &IndexedTransaction,
    ) -> Result<(), TransactionError>
    where
        T: TransactionOutputProvider,
    {
        // let's do preverification first
        let deployments = BlockDeployments::new(
            &self.deployments,
            height,
            block_header_provider,
            &self.consensus,
        );
        let tx_verifier = MemoryPoolTransactionVerifier::new(&transaction, &self.consensus);
        try!(tx_verifier.check());

        let canon_tx = CanonTransaction::new(&transaction);
        // now let's do full verification
        let noop = NoopStore;
        let output_store = DuplexTransactionOutputProvider::new(prevout_provider, &noop);
        let tx_acceptor = MemoryPoolTransactionAcceptor::new(
            self.store.as_transaction_meta_provider(),
            output_store,
            self.store.as_nullifier_tracker(),
            &self.consensus,
            canon_tx,
            height,
            time,
            &deployments,
            self.store.as_tree_state_provider(),
        );
        tx_acceptor.check()
    }
}

impl Verify for BackwardsCompatibleChainVerifier {
    fn verify(&self, level: VerificationLevel, block: &IndexedBlock) -> Result<(), Error> {
        let result = self.verify_block(level, block);
        trace!(
            target: "verification", "Block {} (transactions: {}) verification finished. Result {:?}",
            block.hash().to_reversed_str(),
            block.transactions.len(),
            result,
        );
        result
    }
}

#[cfg(test)]
mod tests {
    extern crate zebra_test_data;

    use super::BackwardsCompatibleChainVerifier as ChainVerifier;
    use std::sync::Arc;
    use zebra_chain::IndexedBlock;
    use zebra_db::BlockChainDatabase;
    use zebra_network::{ConsensusParams, Network};
    use zebra_script;
    use zebra_storage::Error as DBError;
    use {Error, TransactionError, VerificationLevel, Verify};

    #[test]
    fn verify_orphan() {
        let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![
            zebra_test_data::genesis().into(),
        ]));
        let b2 = zebra_test_data::block_h2().into();
        let verifier = ChainVerifier::new(storage, ConsensusParams::new(Network::Unitest));
        assert_eq!(
            Err(Error::Database(DBError::UnknownParent)),
            verifier.verify(VerificationLevel::FULL, &b2)
        );
    }

    #[test]
    fn verify_smoky() {
        let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![
            zebra_test_data::genesis().into(),
        ]));
        let b1 = zebra_test_data::block_h1();
        let verifier = ChainVerifier::new(storage, ConsensusParams::new(Network::Mainnet));
        assert_eq!(verifier.verify(VerificationLevel::FULL, &b1.into()), Ok(()));
    }

    #[test]
    fn first_tx() {
        let storage = BlockChainDatabase::init_test_chain(vec![
            zebra_test_data::block_h0().into(),
            zebra_test_data::block_h1().into(),
        ]);
        let b1 = zebra_test_data::block_h2();
        let verifier =
            ChainVerifier::new(Arc::new(storage), ConsensusParams::new(Network::Mainnet));
        assert_eq!(verifier.verify(VerificationLevel::FULL, &b1.into()), Ok(()));
    }

    #[test]
    fn coinbase_maturity() {
        let consensus = ConsensusParams::new(Network::Unitest);
        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let genesis_coinbase = genesis.transactions()[0].hash();

        let block = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .founder_reward(&consensus, 1)
            .output()
            .value(1)
            .build()
            .build()
            .transaction()
            .input()
            .hash(genesis_coinbase)
            .build()
            .output()
            .value(2)
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build();

        let verifier = ChainVerifier::new(Arc::new(storage), consensus);

        let expected = Err(Error::Transaction(1, TransactionError::Maturity));

        assert_eq!(
            expected,
            verifier.verify(VerificationLevel::FULL, &block.into())
        );
    }

    #[test]
    fn non_coinbase_happy() {
        let consensus = ConsensusParams::new(Network::Unitest);

        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(1)
            .build()
            .build()
            .transaction()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let reference_tx = genesis.transactions()[1].hash();

        let block = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .founder_reward(&consensus, 1)
            .output()
            .value(2)
            .build()
            .build()
            .transaction()
            .input()
            .hash(reference_tx)
            .build()
            .output()
            .value(1)
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build();

        let verifier = ChainVerifier::new(Arc::new(storage), consensus);
        assert_eq!(
            verifier.verify(VerificationLevel::FULL, &block.into()),
            Ok(())
        );
    }

    #[test]
    fn transaction_references_same_block_happy() {
        let consensus = ConsensusParams::new(Network::Unitest);

        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(1)
            .build()
            .build()
            .transaction()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let first_tx_hash = genesis.transactions()[1].hash();

        let block = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .founder_reward(&consensus, 1)
            .output()
            .value(2)
            .build()
            .build()
            .transaction()
            .input()
            .hash(first_tx_hash)
            .build()
            .output()
            .value(30)
            .build()
            .output()
            .value(20)
            .build()
            .build()
            .derived_transaction(1, 0)
            .output()
            .value(30)
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build();

        let verifier = ChainVerifier::new(Arc::new(storage), consensus);
        assert!(verifier
            .verify(VerificationLevel::FULL, &block.into())
            .is_ok());
    }

    #[test]
    fn transaction_references_same_block_overspend() {
        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(1)
            .build()
            .build()
            .transaction()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let first_tx_hash = genesis.transactions()[1].hash();

        let block = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(2)
            .build()
            .build()
            .transaction()
            .input()
            .hash(first_tx_hash)
            .build()
            .output()
            .value(19)
            .build()
            .output()
            .value(31)
            .build()
            .build()
            .derived_transaction(1, 0)
            .output()
            .value(20)
            .build()
            .build()
            .derived_transaction(1, 1)
            .output()
            .value(20)
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build();

        let verifier =
            ChainVerifier::new(Arc::new(storage), ConsensusParams::new(Network::Unitest));

        let expected = Err(Error::Transaction(2, TransactionError::Overspend));
        assert_eq!(
            expected,
            verifier.verify(VerificationLevel::FULL, &block.into())
        );
    }

    #[test]
    #[ignore]
    fn coinbase_happy() {
        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let genesis_coinbase = genesis.transactions()[0].hash();

        // waiting 100 blocks for genesis coinbase to become valid
        for _ in 0..100 {
            let block: IndexedBlock = zebra_test_data::block_builder()
                .transaction()
                .coinbase()
                .build()
                .merkled_header()
                .parent(genesis.hash())
                .build()
                .build()
                .into();
            let hash = block.hash().clone();
            storage
                .insert(block)
                .expect("All dummy blocks should be inserted");
            storage.canonize(&hash).unwrap();
        }

        let best_hash = storage.best_block().hash;

        let block = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .build()
            .transaction()
            .input()
            .hash(genesis_coinbase.clone())
            .build()
            .build()
            .merkled_header()
            .parent(best_hash)
            .build()
            .build();

        let verifier =
            ChainVerifier::new(Arc::new(storage), ConsensusParams::new(Network::Unitest));
        assert!(verifier
            .verify(VerificationLevel::FULL, &block.into())
            .is_ok());
    }

    #[test]
    fn absoulte_sigops_overflow_block() {
        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .build()
            .transaction()
            .output()
            .value(50)
            .build()
            .build()
            .merkled_header()
            .build()
            .build();

        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
        let reference_tx = genesis.transactions()[1].hash();

        let mut builder_tx1 = zebra_script::Builder::default();
        for _ in 0..81000 {
            builder_tx1 = builder_tx1.push_opcode(zebra_script::Opcode::OP_CHECKSIG)
        }

        let mut builder_tx2 = zebra_script::Builder::default();
        for _ in 0..81001 {
            builder_tx2 = builder_tx2.push_opcode(zebra_script::Opcode::OP_CHECKSIG)
        }

        let block: IndexedBlock = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .build()
            .transaction()
            .input()
            .hash(reference_tx.clone())
            .signature_bytes(builder_tx1.into_script().to_bytes())
            .build()
            .build()
            .transaction()
            .input()
            .hash(reference_tx)
            .signature_bytes(builder_tx2.into_script().to_bytes())
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build()
            .into();

        let verifier =
            ChainVerifier::new(Arc::new(storage), ConsensusParams::new(Network::Unitest));
        let expected = Err(Error::MaximumSigops);
        assert_eq!(
            expected,
            verifier.verify(VerificationLevel::FULL, &block.into())
        );
    }

    #[test]
    fn coinbase_overspend() {
        let genesis = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .build()
            .merkled_header()
            .build()
            .build();
        let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);

        let block: IndexedBlock = zebra_test_data::block_builder()
            .transaction()
            .coinbase()
            .output()
            .value(1250000001)
            .build()
            .build()
            .merkled_header()
            .parent(genesis.hash())
            .build()
            .build()
            .into();

        let verifier =
            ChainVerifier::new(Arc::new(storage), ConsensusParams::new(Network::Unitest));

        let expected = Err(Error::CoinbaseOverspend {
            expected_max: 1250000000,
            actual: 1250000001,
        });

        assert_eq!(
            expected,
            verifier.verify(VerificationLevel::FULL, &block.into())
        );
    }
}
