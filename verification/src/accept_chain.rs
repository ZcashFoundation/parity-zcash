use crate::accept_block::BlockAcceptor;
use crate::accept_header::HeaderAcceptor;
use crate::accept_transaction::TransactionAcceptor;
use crate::canon::CanonBlock;
use crate::deployments::BlockDeployments;
use crate::error::Error;
use network::ConsensusParams;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use storage::{
    BlockHeaderProvider, DuplexTransactionOutputProvider, NullifierTracker,
    TransactionMetaProvider, TransactionOutputProvider, TreeStateProvider,
};
use crate::VerificationLevel;

pub struct ChainAcceptor<'a> {
    pub block: BlockAcceptor<'a>,
    pub header: HeaderAcceptor<'a>,
    pub transactions: Vec<TransactionAcceptor<'a>>,
}

impl<'a> ChainAcceptor<'a> {
    pub fn new(
        tx_out_provider: &'a TransactionOutputProvider,
        tx_meta_provider: &'a TransactionMetaProvider,
        header_provider: &'a BlockHeaderProvider,
        tree_state_provider: &'a TreeStateProvider,
        nullifier_tracker: &'a NullifierTracker,
        consensus: &'a ConsensusParams,
        verification_level: VerificationLevel,
        block: CanonBlock<'a>,
        height: u32,
        time: u32,
        deployments: &'a BlockDeployments,
    ) -> Self {
        trace!(target: "verification", "Block verification {}", block.hash().to_reversed_str());
        let output_store = DuplexTransactionOutputProvider::new(tx_out_provider, block.raw());

        ChainAcceptor {
            block: BlockAcceptor::new(
                tx_out_provider,
                tree_state_provider,
                consensus,
                block,
                height,
                deployments,
                header_provider,
            ),
            header: HeaderAcceptor::new(
                header_provider,
                consensus,
                block.header(),
                height,
                time,
                deployments,
            ),
            transactions: block
                .transactions()
                .into_iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    TransactionAcceptor::new(
                        tx_meta_provider,
                        output_store,
                        nullifier_tracker,
                        consensus,
                        tx,
                        verification_level,
                        height,
                        block.header.raw.time,
                        tx_index,
                        deployments,
                        tree_state_provider,
                    )
                })
                .collect(),
        }
    }

    pub fn check(&self) -> Result<(), Error> {
        r#try!(self.block.check());
        r#try!(self.header.check());
        r#try!(self.check_transactions());
        Ok(())
    }

    fn check_transactions(&self) -> Result<(), Error> {
        self.transactions
            .par_iter()
            .enumerate()
            .fold(
                || Ok(()),
                |result, (index, tx)| {
                    result.and_then(|_| tx.check().map_err(|err| Error::Transaction(index, err)))
                },
            )
            .reduce(|| Ok(()), |acc, check| acc.and(check))
    }
}
