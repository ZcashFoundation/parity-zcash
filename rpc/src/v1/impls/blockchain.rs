use chain::OutPoint;
use global_script::Script;
use jsonrpc_core::Error;
use keys::{self, Address};
use network::{ConsensusParams, Network};
use primitives::hash::H256 as GlobalH256;
use ser::serialize;
use storage;
use v1::helpers::errors::{
    block_at_height_not_found, block_not_found, invalid_params, transaction_not_found,
    transaction_of_side_branch, transaction_output_not_found,
};
use v1::traits::BlockChain;
use v1::types::GetTxOutSetInfoResponse;
use v1::types::H256;
use v1::types::{BlockRef, GetBlockResponse, RawBlock, VerboseBlock};
use v1::types::{GetTxOutResponse, TransactionOutputScript};
use verification;

pub struct BlockChainClient<T: BlockChainClientCoreApi> {
    core: T,
}

pub trait BlockChainClientCoreApi: Send + Sync + 'static {
    fn best_block_hash(&self) -> GlobalH256;
    fn block_count(&self) -> u32;
    fn block_hash(&self, height: u32) -> Option<GlobalH256>;
    fn difficulty(&self) -> f64;
    fn raw_block(&self, hash: GlobalH256) -> Option<RawBlock>;
    fn verbose_block(&self, hash: GlobalH256) -> Option<VerboseBlock>;
    fn verbose_transaction_out(&self, prev_out: OutPoint) -> Result<GetTxOutResponse, Error>;
}

pub struct BlockChainClientCore {
    consensus: ConsensusParams,
    storage: storage::SharedStore,
}

impl BlockChainClientCore {
    pub fn new(consensus: ConsensusParams, storage: storage::SharedStore) -> Self {
        BlockChainClientCore {
            consensus: consensus,
            storage: storage,
        }
    }
}

impl BlockChainClientCoreApi for BlockChainClientCore {
    fn best_block_hash(&self) -> GlobalH256 {
        self.storage.best_block().hash
    }

    fn block_count(&self) -> u32 {
        self.storage.best_block().number
    }

    fn block_hash(&self, height: u32) -> Option<GlobalH256> {
        self.storage.block_hash(height)
    }

    fn difficulty(&self) -> f64 {
        let best_block = self.storage.best_block();
        let now = ::time::get_time().sec as u32;

        let next_work_required = verification::work_required(
            best_block.hash,
            now,
            best_block.number + 1,
            self.storage.as_block_header_provider(),
            &self.consensus,
        );

        next_work_required.to_f64(self.consensus.network.max_bits().into())
    }

    fn raw_block(&self, hash: GlobalH256) -> Option<RawBlock> {
        self.storage
            .block(hash.into())
            .map(|block| serialize(&block.to_raw_block()).into())
    }

    fn verbose_block(&self, hash: GlobalH256) -> Option<VerboseBlock> {
        self.storage.block(hash.into()).map(|block| {
            let height = self.storage.block_number(block.hash());
            let confirmations = match height {
                Some(block_number) => (self.storage.best_block().number - block_number + 1) as i64,
                None => -1,
            };
            let block_size = block.size();

            VerboseBlock {
                confirmations: confirmations,
                size: block_size as u32,
                height: height,
                difficulty: block
                    .header
                    .raw
                    .bits
                    .to_f64(self.consensus.network.max_bits().into()),
                previousblockhash: Some(block.header.raw.previous_header_hash.clone().into()),
                nextblockhash: height
                    .and_then(|h| self.storage.block_hash(h + 1).map(|h| h.into())),
                bits: block.header.raw.bits.into(),
                hash: block.hash().clone().into(),
                merkleroot: block.header.raw.merkle_root_hash.clone().into(),
                finalsaplingroot: block.header.raw.final_sapling_root.into(),
                nonce: block.header.raw.nonce.clone().into(),
                time: block.header.raw.time,
                tx: block
                    .transactions
                    .into_iter()
                    .map(|t| t.hash.into())
                    .collect(),
                version: block.header.raw.version,
            }
        })
    }

    fn verbose_transaction_out(&self, prev_out: OutPoint) -> Result<GetTxOutResponse, Error> {
        let transaction = match self.storage.transaction(&prev_out.hash) {
            Some(transaction) => transaction,
            // no transaction => no response
            None => return Err(transaction_not_found(prev_out.hash)),
        };

        if prev_out.index >= transaction.raw.outputs.len() as u32 {
            return Err(transaction_output_not_found(prev_out));
        }

        let meta = match self.storage.transaction_meta(&prev_out.hash) {
            Some(meta) => meta,
            // not in the main branch => no response
            None => return Err(transaction_of_side_branch(prev_out.hash)),
        };

        let block_header = match self.storage.block_header(meta.height().into()) {
            Some(block_header) => block_header,
            // this is possible during reorgs
            None => return Err(transaction_not_found(prev_out.hash)),
        };

        let best_block = self.storage.best_block();
        if best_block.number < meta.height() {
            // this is possible during reorgs
            return Err(transaction_not_found(prev_out.hash));
        }

        let ref script_bytes = transaction.raw.outputs[prev_out.index as usize].script_pubkey;
        let script: Script = script_bytes.clone().into();
        let script_asm = format!("{}", script);
        let script_addresses = script.extract_destinations().unwrap_or(vec![]);

        Ok(GetTxOutResponse {
            bestblock: block_header.hash.into(),
            confirmations: best_block.number - meta.height() + 1,
            value: 0.00000001f64 * (transaction.raw.outputs[prev_out.index as usize].value as f64),
            script: TransactionOutputScript {
                asm: script_asm,
                hex: script_bytes.clone().into(),
                req_sigs: script.num_signatures_required() as u32,
                script_type: script.script_type().into(),
                addresses: script_addresses
                    .into_iter()
                    .map(|a| Address {
                        network: match self.consensus.network {
                            Network::Mainnet => keys::Network::Mainnet,
                            // there's no correct choices for Regtests && Other networks
                            // => let's just make Testnet key
                            _ => keys::Network::Testnet,
                        },
                        hash: a.hash,
                        kind: a.kind,
                    })
                    .collect(),
            },
            version: transaction.raw.version,
            coinbase: transaction.raw.is_coinbase(),
        })
    }
}

impl<T> BlockChainClient<T>
where
    T: BlockChainClientCoreApi,
{
    pub fn new(core: T) -> Self {
        BlockChainClient { core: core }
    }
}

impl<T> BlockChain for BlockChainClient<T>
where
    T: BlockChainClientCoreApi,
{
    fn best_block_hash(&self) -> Result<H256, Error> {
        Ok(self.core.best_block_hash().reversed().into())
    }

    fn block_count(&self) -> Result<u32, Error> {
        Ok(self.core.block_count())
    }

    fn block_hash(&self, height: u32) -> Result<H256, Error> {
        self.core
            .block_hash(height)
            .map(|h| h.reversed().into())
            .ok_or(block_at_height_not_found(height))
    }

    fn difficulty(&self) -> Result<f64, Error> {
        Ok(self.core.difficulty())
    }

    fn block(&self, block: BlockRef, verbosity: Option<u8>) -> Result<GetBlockResponse, Error> {
        let global_hash = match block {
            BlockRef::Number(number) => self
                .core
                .block_hash(number)
                .ok_or(block_not_found(number))?,
            BlockRef::Hash(hash) => {
                let h: GlobalH256 = hash.into();
                h.reversed()
            }
        };

        match verbosity {
            // if verbosity is 0, returns a string that is serialized, hex-encoded data for the block.
            Some(0) => self
                .core
                .raw_block(global_hash)
                .map(GetBlockResponse::Raw)
                .ok_or(block_not_found(global_hash.reversed())),
            // if verbosity is 1, returns an Object with information about the block.
            None | Some(1) => {
                let verbose_block = self.core.verbose_block(global_hash);
                if let Some(mut verbose_block) = verbose_block {
                    verbose_block.previousblockhash =
                        verbose_block.previousblockhash.map(|h| h.reversed());
                    verbose_block.nextblockhash = verbose_block.nextblockhash.map(|h| h.reversed());
                    verbose_block.hash = verbose_block.hash.reversed();
                    verbose_block.merkleroot = verbose_block.merkleroot.reversed();
                    verbose_block.finalsaplingroot = verbose_block.finalsaplingroot.reversed();
                    verbose_block.tx = verbose_block.tx.into_iter().map(|h| h.reversed()).collect();
                    Some(GetBlockResponse::Verbose(verbose_block))
                } else {
                    None
                }
                .ok_or(block_not_found(global_hash.reversed()))
            }
            // if verbosity is 2, returns an Object with information about the block and information about each transaction.
            // we do not (yet?) support getrawtransaction call => nothing to return
            Some(2) => rpc_unimplemented!(),
            _ => Err(invalid_params("verbosity", verbosity)),
        }
    }

    fn transaction_out(
        &self,
        transaction_hash: H256,
        out_index: u32,
        _include_mempool: Option<bool>,
    ) -> Result<GetTxOutResponse, Error> {
        // TODO: include_mempool
        let transaction_hash: GlobalH256 = transaction_hash.into();
        self.core
            .verbose_transaction_out(OutPoint {
                hash: transaction_hash.reversed(),
                index: out_index,
            })
            .map(|mut response| {
                response.bestblock = response.bestblock.reversed();
                response
            })
    }

    fn transaction_out_set_info(&self) -> Result<GetTxOutSetInfoResponse, Error> {
        rpc_unimplemented!()
    }
}

#[cfg(test)]
pub mod tests {
    extern crate test_data;

    use super::*;
    use chain::OutPoint;
    use db::BlockChainDatabase;
    use jsonrpc_core::Error;
    use jsonrpc_core::IoHandler;
    use network::Network;
    use primitives::bytes::Bytes as GlobalBytes;
    use primitives::hash::H256 as GlobalH256;
    use std::sync::Arc;
    use v1::helpers::errors::block_not_found;
    use v1::traits::BlockChain;
    use v1::types::Bytes;
    use v1::types::ScriptType;
    use v1::types::H256;
    use v1::types::{GetTxOutResponse, TransactionOutputScript};
    use v1::types::{RawBlock, VerboseBlock};

    #[derive(Default)]
    struct SuccessBlockChainClientCore;
    #[derive(Default)]
    struct ErrorBlockChainClientCore;

    impl BlockChainClientCoreApi for SuccessBlockChainClientCore {
        fn best_block_hash(&self) -> GlobalH256 {
            test_data::genesis().hash()
        }

        fn block_count(&self) -> u32 {
            1
        }

        fn block_hash(&self, _height: u32) -> Option<GlobalH256> {
            Some(test_data::genesis().hash())
        }

        fn difficulty(&self) -> f64 {
            1f64
        }

        fn raw_block(&self, _hash: GlobalH256) -> Option<RawBlock> {
            let b2_bytes: GlobalBytes = "010000004860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9bb0bc6649ffff001d08d2bd610101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d010bffffffff0100f2052a010000004341047211a824f55b505228e4c3d5194c1fcfaa15a456abdf37f9b9d97a4040afc073dee6c89064984f03385237d92167c13e236446b417ab79a0fcae412ae3316b77ac00000000".into();
            Some(RawBlock::from(b2_bytes))
        }

        fn verbose_block(&self, _hash: GlobalH256) -> Option<VerboseBlock> {
            // https://blockexplorer.com/block/000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd
            // https://blockchain.info/ru/block/000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd
            // https://webbtc.com/block/000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd.json
            Some(VerboseBlock {
                hash: "bddd99ccfda39da1b108ce1a5d70038d0a967bacb68b6b63065f626a00000000".into(),
                confirmations: 1, // h2
                size: 215,
                height: Some(2),
                version: 1,
                merkleroot: "d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9b"
                    .into(),
                tx: vec!["d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9b".into()],
                time: 1231469744,
                nonce: 42.into(),
                bits: 486604799,
                difficulty: 1.0,
                previousblockhash: Some(
                    "4860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000".into(),
                ),
                nextblockhash: None,
                finalsaplingroot:
                    "a5556cd346010000000000000000000000000000000000000000000000000002".into(),
            })
        }

        fn verbose_transaction_out(&self, _prev_out: OutPoint) -> Result<GetTxOutResponse, Error> {
            Ok(GetTxOutResponse {
                bestblock: H256::from(0x56),
                confirmations: 777,
                value: 100000.56,
                script: TransactionOutputScript {
                    asm: "Hello, world!!!".to_owned(),
                    hex: Bytes::new(vec![1, 2, 3, 4]),
                    req_sigs: 777,
                    script_type: ScriptType::Multisig,
                    addresses: vec![
                        "t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi".into(),
                        "t2N9PH9Wk9xjqYg9iin1Ua3aekJqfAtE543".into(),
                    ],
                },
                version: 33,
                coinbase: false,
            })
        }
    }

    impl BlockChainClientCoreApi for ErrorBlockChainClientCore {
        fn best_block_hash(&self) -> GlobalH256 {
            test_data::genesis().hash()
        }

        fn block_count(&self) -> u32 {
            1
        }

        fn block_hash(&self, _height: u32) -> Option<GlobalH256> {
            None
        }

        fn difficulty(&self) -> f64 {
            1f64
        }

        fn raw_block(&self, _hash: GlobalH256) -> Option<RawBlock> {
            None
        }

        fn verbose_block(&self, _hash: GlobalH256) -> Option<VerboseBlock> {
            None
        }

        fn verbose_transaction_out(&self, prev_out: OutPoint) -> Result<GetTxOutResponse, Error> {
            Err(block_not_found(prev_out.hash))
        }
    }

    #[test]
    fn best_block_hash_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getbestblockhash",
				"params": [],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","result":"00040fe8ec8471911baa1db1266ea15dd06b4a8a5c453883c000b031973dce08","id":1}"#);
    }

    #[test]
    fn block_count_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblockcount",
				"params": [],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","result":1,"id":1}"#);
    }

    #[test]
    fn block_hash_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblockhash",
				"params": [0],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","result":"00040fe8ec8471911baa1db1266ea15dd06b4a8a5c453883c000b031973dce08","id":1}"#);
    }

    #[test]
    fn block_hash_error() {
        let client = BlockChainClient::new(ErrorBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblockhash",
				"params": [0],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","error":{"code":-32099,"message":"Block at given height is not found","data":"0"},"id":1}"#);
    }

    #[test]
    fn difficulty_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getdifficulty",
				"params": [],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","result":1.0,"id":1}"#);
    }

    #[test]
    fn verbose_block_contents() {
        let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![
            test_data::genesis().into(),
            test_data::block_h1().into(),
            test_data::block_h2().into(),
        ]));

        let core = BlockChainClientCore::new(ConsensusParams::new(Network::Mainnet), storage);

        // get info on block #1:
        // https://zcash.blockexplorer.com/block/0007bc227e1c57a4a70e237cad00e7b7ce565155ab49166bc57397a26d339283
        let verbose_block = core.verbose_block(
            "8392336da29773c56b1649ab555156ceb7e700ad7c230ea7a4571c7e22bc0700".into(),
        );
        assert_eq!(
            verbose_block,
            Some(VerboseBlock {
                hash: "8392336da29773c56b1649ab555156ceb7e700ad7c230ea7a4571c7e22bc0700".into(),
                confirmations: 2, // h1 + h2
                size: 1617,
                height: Some(1),
                version: 4,
                merkleroot: "0946edb9c083c9942d92305444527765fad789c438c717783276a9f7fbf61b85"
                    .into(),
                tx: vec!["0946edb9c083c9942d92305444527765fad789c438c717783276a9f7fbf61b85".into()],
                time: 1477671596,
                nonce: "7534e8cf161ff2e49d54bdb3bfbcde8cdbf2fc5963c9ec7d86aed4a67e975790".into(),
                bits: 520617983,
                difficulty: 1.0,
                previousblockhash: Some(
                    "08ce3d9731b000c08338455c8a4a6bd05da16e26b11daa1b917184ece80f0400".into()
                ),
                nextblockhash: Some(
                    "ed73e297d7c51cb8dc53fc2213d7e2e3f116eb4f26434496fc1926906ca20200".into()
                ),
                finalsaplingroot:
                    "0000000000000000000000000000000000000000000000000000000000000000".into(),
            })
        );

        // get info on block #2:
        // https://zcash.blockexplorer.com/block/0002a26c902619fc964443264feb16f1e3e2d71322fc53dcb81cc5d797e273ed
        let verbose_block = core.verbose_block(
            "ed73e297d7c51cb8dc53fc2213d7e2e3f116eb4f26434496fc1926906ca20200".into(),
        );
        assert_eq!(
            verbose_block,
            Some(VerboseBlock {
                hash: "ed73e297d7c51cb8dc53fc2213d7e2e3f116eb4f26434496fc1926906ca20200".into(),
                confirmations: 1, // h2
                size: 1617,
                height: Some(2),
                version: 4,
                merkleroot: "f4b084a7c2fc5a5aa2985f2bcb1d4a9a65562a589d628b0d869c5f1c8dd07489"
                    .into(),
                tx: vec!["f4b084a7c2fc5a5aa2985f2bcb1d4a9a65562a589d628b0d869c5f1c8dd07489".into()],
                time: 1477671626,
                nonce: "a5556cd346010000000000000000000000000000000000000000000000000002".into(),
                bits: 520617983,
                difficulty: 1.0,
                previousblockhash: Some(
                    "8392336da29773c56b1649ab555156ceb7e700ad7c230ea7a4571c7e22bc0700".into()
                ),
                nextblockhash: None,
                finalsaplingroot:
                    "0000000000000000000000000000000000000000000000000000000000000000".into(),
            })
        );
    }

    #[test]
    fn raw_block_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let expected = r#"{"jsonrpc":"2.0","result":"010000004860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9bb0bc6649ffff001d08d2bd610101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d010bffffffff0100f2052a010000004341047211a824f55b505228e4c3d5194c1fcfaa15a456abdf37f9b9d97a4040afc073dee6c89064984f03385237d92167c13e236446b417ab79a0fcae412ae3316b77ac00000000","id":1}"#;

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblock",
				"params": ["000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd", 0],
				"id": 1
			}"#),
            )
            .unwrap();
        assert_eq!(&sample, expected);
    }

    #[test]
    fn raw_block_error() {
        let client = BlockChainClient::new(ErrorBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblock",
				"params": ["000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd", 0],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","error":{"code":-32099,"message":"Block with given hash is not found","data":"000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd"},"id":1}"#);
    }

    #[test]
    fn verbose_block_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let expected = r#"{"jsonrpc":"2.0","result":{"bits":486604799,"confirmations":1,"difficulty":1.0,"finalsaplingroot":"02000000000000000000000000000000000000000000000000000146d36c55a5","hash":"000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd","height":2,"merkleroot":"9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5","nextblockhash":null,"nonce":"2a00000000000000000000000000000000000000000000000000000000000000","previousblockhash":"00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048","size":215,"time":1231469744,"tx":["9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5"],"version":1},"id":1}"#;

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblock",
				"params": ["000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd",1],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, expected);

        // try without optional parameter
        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblock",
				"params": ["000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd"],
				"id": 1
			}"#),
            )
            .unwrap();
        assert_eq!(&sample, expected);
    }

    #[test]
    fn verbose_block_error() {
        let client = BlockChainClient::new(ErrorBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "getblock",
				"params": ["000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd", 1],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","error":{"code":-32099,"message":"Block with given hash is not found","data":"000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd"},"id":1}"#);
    }

    #[test]
    fn verbose_transaction_out_contents() {
        let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![
            test_data::genesis().into(),
            test_data::block_h1().into(),
        ]));
        let core = BlockChainClientCore::new(ConsensusParams::new(Network::Mainnet), storage);

        // get info on tx from block#1:
        // https://zcash.blockexplorer.com/tx/851bf6fbf7a976327817c738c489d7fa657752445430922d94c983c0b9ed4609
        let verbose_transaction_out = core.verbose_transaction_out(OutPoint {
            hash: "0946edb9c083c9942d92305444527765fad789c438c717783276a9f7fbf61b85".into(),
            index: 0,
        });
        assert_eq!(verbose_transaction_out, Ok(GetTxOutResponse {
				bestblock: "8392336da29773c56b1649ab555156ceb7e700ad7c230ea7a4571c7e22bc0700".into(),
				confirmations: 1,
				value: 0.0005,
				script: TransactionOutputScript {
					asm: "OP_PUSHBYTES_33 0x027a46eb513588b01b37ea24303f4b628afd12cc20df789fede0921e43cad3e875\nOP_CHECKSIG\n".to_owned(),
					hex: Bytes::from("21027a46eb513588b01b37ea24303f4b628afd12cc20df789fede0921e43cad3e875ac"),
					req_sigs: 1,
					script_type: ScriptType::PubKey,
					addresses: vec!["t1KstPVzcNEK4ZeauQ6cogoqxQBMDSiRnGr".into()]
				},
				version: 1,
				coinbase: true
			}));
    }

    #[test]
    fn transaction_out_success() {
        let client = BlockChainClient::new(SuccessBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "gettxout",
				"params": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", 0],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","result":{"bestblock":"0000000000000000000000000000000000000000000000000000000000000056","coinbase":false,"confirmations":777,"scriptPubKey":{"addresses":["t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi","t2N9PH9Wk9xjqYg9iin1Ua3aekJqfAtE543"],"asm":"Hello, world!!!","hex":"01020304","reqSigs":777,"type":"multisig"},"value":100000.56,"version":33},"id":1}"#);
    }

    #[test]
    fn transaction_out_failure() {
        let client = BlockChainClient::new(ErrorBlockChainClientCore::default());
        let mut handler = IoHandler::new();
        handler.extend_with(client.to_delegate());

        let sample = handler
            .handle_request_sync(
                &(r#"
			{
				"jsonrpc": "2.0",
				"method": "gettxout",
				"params": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", 0],
				"id": 1
			}"#),
            )
            .unwrap();

        assert_eq!(&sample, r#"{"jsonrpc":"2.0","error":{"code":-32099,"message":"Block with given hash is not found","data":"3ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a"},"id":1}"#);
    }
}
