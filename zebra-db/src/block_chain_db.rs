use bytes::Bytes;
use hash::H256;
use kv::{
    AutoFlushingOverlayDatabase, CacheDatabase, DatabaseConfig, DiskDatabase, Key, KeyState,
    KeyValue, KeyValueDatabase, MemoryDatabase, OverlayDatabase, Transaction as DBTransaction,
    Value,
};
use kv::{
    COL_BLOCK_HASHES, COL_BLOCK_HEADERS, COL_BLOCK_NUMBERS, COL_BLOCK_TRANSACTIONS, COL_COUNT,
    COL_SAPLING_NULLIFIERS, COL_SPROUT_BLOCK_ROOTS, COL_SPROUT_NULLIFIERS, COL_TRANSACTIONS,
    COL_TRANSACTIONS_META, COL_TREE_STATES,
};
use parking_lot::RwLock;
use ser::{deserialize, serialize, List};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use zebra_chain::{
    IndexedBlock, IndexedBlockHeader, IndexedTransaction, OutPoint, TransactionOutput,
};
use zebra_storage::{
    BestBlock, BlockChain, BlockHeaderProvider, BlockOrigin, BlockProvider, BlockRef, CanonStore,
    EpochRef, EpochTag, Error, ForkChain, Forkable, NullifierTracker, SaplingTreeState,
    SideChainOrigin, SproutTreeState, Store, TransactionMeta, TransactionMetaProvider,
    TransactionOutputProvider, TransactionProvider, TreeStateProvider,
};

const KEY_BEST_BLOCK_NUMBER: &'static str = "best_block_number";
const KEY_BEST_BLOCK_HASH: &'static str = "best_block_hash";

const MAX_FORK_ROUTE_PRESET: usize = 2048;

pub struct BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    best_block: RwLock<BestBlock>,
    db: T,
}

pub struct ForkChainDatabase<'a, T>
where
    T: 'a + KeyValueDatabase,
{
    blockchain: BlockChainDatabase<OverlayDatabase<'a, T>>,
}

impl<'a, T> ForkChain for ForkChainDatabase<'a, T>
where
    T: KeyValueDatabase,
{
    fn store(&self) -> &Store {
        &self.blockchain
    }

    fn flush(&self) -> Result<(), Error> {
        self.blockchain.db.flush().map_err(Error::DatabaseError)
    }
}

mod cache {
    pub const CACHE_TRANSACTIONS: u32 = 20;
    pub const CACHE_TRANSACTION_META: u32 = 20;
    pub const CACHE_HEADERS: u32 = 15;
    pub const CACHE_BLOCK_HASHES: u32 = 5;
    pub const CACHE_BLOCK_TRANSACTIONS: u32 = 10;
    pub const CACHE_BLOCK_NUMBERS: u32 = 5;
    pub const CACHE_SPROUT_NULLIFIERS: u32 = 5;
    pub const CACHE_SAPLING_NULLIFIERS: u32 = 5;
    pub const CACHE_TREE_STATES: u32 = 10;
    pub const CACHE_SPROUT_BLOCK_ROOTS: u32 = 5;

    pub fn set(cfg: &mut ::kv::DatabaseConfig, total: usize, col: u32, distr: u32) {
        cfg.set_cache(
            Some(col),
            (total as f32 * distr as f32 / 100f32).round() as usize,
        )
    }

    #[test]
    fn total_is_100() {
        assert_eq!(
            100,
            CACHE_TRANSACTIONS
                + CACHE_TRANSACTION_META
                + CACHE_HEADERS
                + CACHE_BLOCK_HASHES
                + CACHE_BLOCK_TRANSACTIONS
                + CACHE_BLOCK_NUMBERS
                + CACHE_SPROUT_NULLIFIERS
                + CACHE_SAPLING_NULLIFIERS
                + CACHE_TREE_STATES
                + CACHE_SPROUT_BLOCK_ROOTS
        );
    }
}

impl BlockChainDatabase<CacheDatabase<AutoFlushingOverlayDatabase<DiskDatabase>>> {
    pub fn open_at_path<P>(path: P, total_cache: usize) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        fs::create_dir_all(path.as_ref()).map_err(|err| Error::DatabaseError(err.to_string()))?;
        let mut cfg = DatabaseConfig::with_columns(Some(COL_COUNT));

        cache::set(
            &mut cfg,
            total_cache,
            COL_TRANSACTIONS,
            cache::CACHE_TRANSACTIONS,
        );
        cache::set(
            &mut cfg,
            total_cache,
            COL_TRANSACTIONS_META,
            cache::CACHE_TRANSACTION_META,
        );
        cache::set(
            &mut cfg,
            total_cache,
            COL_BLOCK_HEADERS,
            cache::CACHE_HEADERS,
        );

        cache::set(
            &mut cfg,
            total_cache,
            COL_BLOCK_HASHES,
            cache::CACHE_BLOCK_HASHES,
        );
        cache::set(
            &mut cfg,
            total_cache,
            COL_BLOCK_TRANSACTIONS,
            cache::CACHE_BLOCK_TRANSACTIONS,
        );
        cache::set(
            &mut cfg,
            total_cache,
            COL_BLOCK_NUMBERS,
            cache::CACHE_BLOCK_NUMBERS,
        );

        cache::set(
            &mut cfg,
            total_cache,
            COL_SPROUT_NULLIFIERS,
            cache::CACHE_SPROUT_NULLIFIERS,
        );
        cache::set(
            &mut cfg,
            total_cache,
            COL_SAPLING_NULLIFIERS,
            cache::CACHE_SAPLING_NULLIFIERS,
        );

        cache::set(
            &mut cfg,
            total_cache,
            COL_TREE_STATES,
            cache::CACHE_TREE_STATES,
        );

        cache::set(
            &mut cfg,
            total_cache,
            COL_SPROUT_BLOCK_ROOTS,
            cache::CACHE_SPROUT_BLOCK_ROOTS,
        );

        cfg.bloom_filters.insert(Some(COL_TRANSACTIONS_META), 32);

        match DiskDatabase::open(cfg, path) {
            Ok(db) => Ok(Self::open_with_cache(db)),
            Err(err) => Err(Error::DatabaseError(err)),
        }
    }
}

impl BlockChainDatabase<MemoryDatabase> {
    pub fn init_test_chain(blocks: Vec<IndexedBlock>) -> Self {
        let store = BlockChainDatabase::open(MemoryDatabase::default());

        for block in blocks {
            let hash = block.hash().clone();
            store.insert(block).unwrap();
            store.canonize(&hash).unwrap();
        }
        store
    }
}

impl<T> BlockChainDatabase<CacheDatabase<AutoFlushingOverlayDatabase<T>>>
where
    T: KeyValueDatabase,
{
    pub fn open_with_cache(db: T) -> Self {
        let db = CacheDatabase::new(AutoFlushingOverlayDatabase::new(db, 50));
        let best_block = Self::read_best_block(&db).unwrap_or_default();
        BlockChainDatabase {
            best_block: RwLock::new(best_block),
            db: db,
        }
    }
}

impl<T> BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn read_best_block(db: &T) -> Option<BestBlock> {
        let best_number = db
            .get(&Key::Meta(KEY_BEST_BLOCK_NUMBER))
            .map(KeyState::into_option)
            .map(|x| x.and_then(Value::as_meta));
        let best_hash = db
            .get(&Key::Meta(KEY_BEST_BLOCK_HASH))
            .map(KeyState::into_option)
            .map(|x| x.and_then(Value::as_meta));

        match (best_number, best_hash) {
            (Ok(None), Ok(None)) => None,
            (Ok(Some(number)), Ok(Some(hash))) => Some(BestBlock {
                number: deserialize(&**number)
                    .expect("Inconsistent DB. Invalid best block number."),
                hash: deserialize(&**hash).expect("Inconsistent DB. Invalid best block hash."),
            }),
            _ => panic!("Inconsistent DB"),
        }
    }

    pub fn open(db: T) -> Self {
        let best_block = Self::read_best_block(&db).unwrap_or_default();
        BlockChainDatabase {
            best_block: RwLock::new(best_block),
            db: db,
        }
    }

    pub fn best_block(&self) -> BestBlock {
        self.best_block.read().clone()
    }

    pub fn fork(&self, side_chain: SideChainOrigin) -> Result<ForkChainDatabase<T>, Error> {
        let overlay = BlockChainDatabase::open(OverlayDatabase::new(&self.db));

        for hash in side_chain.decanonized_route.into_iter().rev() {
            let decanonized_hash = overlay.decanonize()?;
            assert_eq!(hash, decanonized_hash);
        }

        for block_hash in &side_chain.canonized_route {
            overlay.canonize(block_hash)?;
        }

        let fork = ForkChainDatabase {
            blockchain: overlay,
        };

        Ok(fork)
    }

    pub fn switch_to_fork(&self, fork: ForkChainDatabase<T>) -> Result<(), Error> {
        let mut best_block = self.best_block.write();
        *best_block = fork.blockchain.best_block.read().clone();
        fork.blockchain.db.flush().map_err(Error::DatabaseError)
    }

    pub fn block_origin(&self, header: &IndexedBlockHeader) -> Result<BlockOrigin, Error> {
        let best_block = self.best_block.read();
        assert_eq!(
            Some(best_block.hash.clone()),
            self.block_hash(best_block.number)
        );
        if self.contains_block(header.hash.clone().into()) {
            // it does not matter if it's canon chain or side chain block
            return Ok(BlockOrigin::KnownBlock);
        }

        if best_block.hash == header.raw.previous_header_hash {
            return Ok(BlockOrigin::CanonChain {
                block_number: best_block.number + 1,
            });
        }

        if !self.contains_block(header.raw.previous_header_hash.clone().into()) {
            return Err(Error::UnknownParent);
        }

        let mut sidechain_route = Vec::new();
        let mut next_hash = header.raw.previous_header_hash.clone();

        for fork_len in 0..MAX_FORK_ROUTE_PRESET {
            match self.block_number(&next_hash) {
                Some(number) => {
                    let block_number = number + fork_len as u32 + 1;
                    let origin = SideChainOrigin {
                        ancestor: number,
                        canonized_route: sidechain_route.into_iter().rev().collect(),
                        decanonized_route: (number + 1..best_block.number + 1)
                            .into_iter()
                            .filter_map(|decanonized_bn| self.block_hash(decanonized_bn))
                            .collect(),
                        block_number: block_number,
                    };
                    if block_number > best_block.number {
                        return Ok(BlockOrigin::SideChainBecomingCanonChain(origin));
                    } else {
                        return Ok(BlockOrigin::SideChain(origin));
                    }
                }
                None => {
                    sidechain_route.push(next_hash.clone());
                    next_hash = self
                        .block_header(next_hash.into())
                        .expect("not to find orphaned side chain in database; qed")
                        .raw
                        .previous_header_hash;
                }
            }
        }

        Err(Error::AncientFork)
    }

    pub fn insert(&self, block: IndexedBlock) -> Result<(), Error> {
        if self.contains_block(block.hash().clone().into()) {
            return Ok(());
        }

        let parent_hash = block.header.raw.previous_header_hash;
        if !self.contains_block(parent_hash.into()) && !parent_hash.is_zero() {
            return Err(Error::UnknownParent);
        }

        let mut sprout_tree_state = if parent_hash.is_zero() {
            SproutTreeState::new()
        } else {
            self.sprout_tree_at_block(&parent_hash).expect(&format!(
                "Corrupted database - no sprout root for block {}",
                parent_hash
            ))
        };

        let mut sapling_tree_state = if parent_hash.is_zero() {
            SaplingTreeState::new()
        } else {
            self.sapling_tree_at_block(&parent_hash).expect(&format!(
                "Corrupted database - no sapling root for block {}",
                parent_hash
            ))
        };

        let sapling_tree_root = block.header.raw.final_sapling_root;
        let mut update = DBTransaction::new();
        update.insert(KeyValue::BlockHeader(*block.hash(), block.header.raw));
        let tx_hashes = block
            .transactions
            .iter()
            .map(|tx| tx.hash)
            .collect::<Vec<_>>();
        update.insert(KeyValue::BlockTransactions(
            block.header.hash,
            List::from(tx_hashes),
        ));

        for tx in block.transactions.into_iter() {
            if let Some(ref js) = tx.raw.join_split {
                for js_descriptor in js.descriptions.iter() {
                    for commitment in &js_descriptor.commitments[..] {
                        sprout_tree_state
                            .append(H256::from(&commitment[..]))
                            .expect("Appending to a full commitment tree in the block insertion")
                    }
                }
            }

            if let Some(ref sapling) = tx.raw.sapling {
                for out_desc in &sapling.outputs {
                    sapling_tree_state
                        .append(out_desc.note_commitment.into())
                        .expect(
                            "only returns Err if tree is already full;
							sapling tree has height = 32;
							it means that there must be 2^32-1 sapling output descriptions to make it full;
							this should be detected by verification (which preceeds insert);
							qed",
                        );
                }
            }

            update.insert(KeyValue::Transaction(tx.hash, tx.raw));
        }

        let sprout_tree_root = sprout_tree_state.root();
        update.insert(KeyValue::SproutBlockRoot(
            block.header.hash,
            sprout_tree_root,
        ));
        update.insert(KeyValue::SproutTreeState(
            sprout_tree_root,
            sprout_tree_state,
        ));

        // TODO: possible optimization is not to store sapling trees until sapling is activated
        update.insert(KeyValue::SaplingTreeState(
            sapling_tree_root,
            sapling_tree_state,
        ));

        self.db.write(update).map_err(Error::DatabaseError)
    }

    /// Rollbacks single best block.
    fn rollback_best(&self) -> Result<H256, Error> {
        let best_block_hash = self.best_block.read().hash.clone();
        let tx_to_decanonize = self.block_transaction_hashes(best_block_hash.into());
        let decanonized_hash = self.decanonize()?;
        debug_assert_eq!(best_block_hash, decanonized_hash);

        // and now remove decanonized block from database
        // all code currently works in assumption that origin of all blocks is one of:
        // {CanonChain, SideChain, SideChainBecomingCanonChain}
        let mut update = DBTransaction::new();
        update.delete(Key::BlockHeader(decanonized_hash.clone()));
        update.delete(Key::BlockTransactions(decanonized_hash.clone()));
        for tx_hash in tx_to_decanonize {
            update.delete(Key::Transaction(tx_hash));
        }

        self.db.write(update).map_err(Error::DatabaseError)?;

        Ok(self.best_block().hash)
    }

    /// Marks block as a new best block.
    ///
    /// Block must be already inserted into db, and its parent must be current best block.
    /// Updates meta data.
    pub fn canonize(&self, hash: &H256) -> Result<(), Error> {
        let mut best_block = self.best_block.write();
        let block = match self.block(hash.clone().into()) {
            Some(block) => block,
            None => {
                error!(target: "db", "Block is not found during canonization: {}", hash.reversed());
                return Err(Error::CannotCanonize);
            }
        };

        if best_block.hash != block.header.raw.previous_header_hash {
            error!(
                target: "db",
                "Wrong best block during canonization. Best {}, parent: {}",
                best_block.hash.reversed(),
                block.header.raw.previous_header_hash.reversed(),
            );
            return Err(Error::CannotCanonize);
        }

        let new_best_block = BestBlock {
            hash: hash.clone(),
            number: if block.header.raw.previous_header_hash.is_zero() {
                assert_eq!(best_block.number, 0);
                0
            } else {
                best_block.number + 1
            },
        };

        trace!(target: "db", "canonize {:?}", new_best_block);

        let mut update = DBTransaction::new();
        update.insert(KeyValue::BlockHash(
            new_best_block.number,
            new_best_block.hash.clone(),
        ));
        update.insert(KeyValue::BlockNumber(
            new_best_block.hash.clone(),
            new_best_block.number,
        ));
        update.insert(KeyValue::Meta(
            KEY_BEST_BLOCK_HASH,
            serialize(&new_best_block.hash),
        ));
        update.insert(KeyValue::Meta(
            KEY_BEST_BLOCK_NUMBER,
            serialize(&new_best_block.number),
        ));

        let mut modified_meta: HashMap<H256, TransactionMeta> = HashMap::new();
        if let Some(tx) = block.transactions.first() {
            let meta = TransactionMeta::new_coinbase(new_best_block.number, tx.raw.outputs.len());
            modified_meta.insert(tx.hash.clone(), meta);
        }

        for tx in block.transactions.iter().skip(1) {
            modified_meta.insert(
                tx.hash.clone(),
                TransactionMeta::new(new_best_block.number, tx.raw.outputs.len()),
            );

            if let Some(ref js) = tx.raw.join_split {
                for js_descriptor in js.descriptions.iter() {
                    for nullifier in &js_descriptor.nullifiers[..] {
                        let nullifier_key =
                            EpochRef::new(EpochTag::Sprout, H256::from(&nullifier[..]));
                        if self.contains_nullifier(nullifier_key) {
                            error!(target: "db", "Duplicate sprout nullifer during canonization: {:?}", nullifier_key);
                            return Err(Error::CannotCanonize);
                        }
                        update.insert(KeyValue::Nullifier(nullifier_key));
                    }
                }
            }

            if let Some(ref sapling) = tx.raw.sapling {
                for spend in &sapling.spends {
                    let nullifier_key =
                        EpochRef::new(EpochTag::Sapling, H256::from(&spend.nullifier[..]));
                    if self.contains_nullifier(nullifier_key) {
                        error!(target: "db", "Duplicate sapling nullifer during canonization: {:?}", nullifier_key);
                        return Err(Error::CannotCanonize);
                    }
                    update.insert(KeyValue::Nullifier(nullifier_key));
                }
            }

            for input in &tx.raw.inputs {
                use std::collections::hash_map::Entry;

                match modified_meta.entry(input.previous_output.hash.clone()) {
                    Entry::Occupied(mut entry) => {
                        let meta = entry.get_mut();
                        meta.denote_used(input.previous_output.index as usize);
                    }
                    Entry::Vacant(entry) => {
                        let mut meta = self
                            .transaction_meta(&input.previous_output.hash)
                            .ok_or_else(|| {
                                error!(
                                    target: "db",
                                    "Cannot find tx meta during canonization of tx {}: {}/{}",
                                    tx.hash.reversed(),
                                    input.previous_output.hash.reversed(),
                                    input.previous_output.index,
                                );
                                Error::CannotCanonize
                            })?;
                        meta.denote_used(input.previous_output.index as usize);
                        entry.insert(meta);
                    }
                }
            }
        }

        for (hash, meta) in modified_meta.into_iter() {
            update.insert(KeyValue::TransactionMeta(hash, meta));
        }

        self.db.write(update).map_err(Error::DatabaseError)?;
        *best_block = new_best_block;
        Ok(())
    }

    pub fn decanonize(&self) -> Result<H256, Error> {
        let mut best_block = self.best_block.write();
        let block = match self.block(best_block.hash.clone().into()) {
            Some(block) => block,
            None => {
                error!(target: "db", "Block is not found during decanonization: {}", best_block.hash.reversed());
                return Err(Error::CannotDecanonize);
            }
        };
        let block_number = best_block.number;
        let block_hash = best_block.hash.clone();

        let new_best_block = BestBlock {
            hash: block.header.raw.previous_header_hash.clone(),
            number: if best_block.number > 0 {
                best_block.number - 1
            } else {
                assert!(block.header.raw.previous_header_hash.is_zero());
                0
            },
        };

        trace!(target: "db", "decanonize, new best: {:?}", new_best_block);

        let mut update = DBTransaction::new();
        update.delete(Key::BlockHash(block_number));
        update.delete(Key::BlockNumber(block_hash.clone()));
        update.insert(KeyValue::Meta(
            KEY_BEST_BLOCK_HASH,
            serialize(&new_best_block.hash),
        ));
        update.insert(KeyValue::Meta(
            KEY_BEST_BLOCK_NUMBER,
            serialize(&new_best_block.number),
        ));

        let mut modified_meta: HashMap<H256, TransactionMeta> = HashMap::new();
        for tx in block.transactions.iter().skip(1) {
            if let Some(ref js) = tx.raw.join_split {
                for js_descriptor in js.descriptions.iter() {
                    for nullifier in &js_descriptor.nullifiers[..] {
                        let nullifier_key =
                            EpochRef::new(EpochTag::Sprout, H256::from(&nullifier[..]));
                        if !self.contains_nullifier(nullifier_key) {
                            error!(target: "db", "cannot decanonize, no sprout nullifier: {:?}", nullifier_key);
                            return Err(Error::CannotDecanonize);
                        }
                        update.delete(Key::Nullifier(nullifier_key));
                    }
                }
            }

            if let Some(ref sapling) = tx.raw.sapling {
                for spend in &sapling.spends {
                    let nullifier_key =
                        EpochRef::new(EpochTag::Sapling, H256::from(&spend.nullifier[..]));
                    if !self.contains_nullifier(nullifier_key) {
                        error!(target: "db", "cannot decanonize, no sapling nullifier: {:?}", nullifier_key);
                        return Err(Error::CannotDecanonize);
                    }
                    update.delete(Key::Nullifier(nullifier_key));
                }
            }

            for input in &tx.raw.inputs {
                use std::collections::hash_map::Entry;

                match modified_meta.entry(input.previous_output.hash.clone()) {
                    Entry::Occupied(mut entry) => {
                        let meta = entry.get_mut();
                        meta.denote_unused(input.previous_output.index as usize);
                    }
                    Entry::Vacant(entry) => {
                        let mut meta = self
                            .transaction_meta(&input.previous_output.hash)
                            .ok_or_else(|| {
                                error!(
                                    target: "db",
                                    "Cannot find tx meta during decanonization of tx {}: {}/{}",
                                    tx.hash.reversed(),
                                    input.previous_output.hash.reversed(),
                                    input.previous_output.index,
                                );
                                Error::CannotDecanonize
                            })?;
                        meta.denote_unused(input.previous_output.index as usize);
                        entry.insert(meta);
                    }
                }
            }
        }

        for (hash, meta) in modified_meta {
            update.insert(KeyValue::TransactionMeta(hash, meta));
        }

        for tx in block.transactions {
            update.delete(Key::TransactionMeta(tx.hash));
        }

        self.db.write(update).map_err(Error::DatabaseError)?;
        *best_block = new_best_block;
        Ok(block_hash)
    }

    fn get(&self, key: Key) -> Option<Value> {
        self.db
            .get(&key)
            .expect("db value to be fine")
            .into_option()
    }

    fn resolve_hash(&self, block_ref: BlockRef) -> Option<H256> {
        match block_ref {
            BlockRef::Number(n) => self.block_hash(n),
            BlockRef::Hash(h) => Some(h),
        }
    }
}

impl<T> BlockHeaderProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn block_header_bytes(&self, block_ref: BlockRef) -> Option<Bytes> {
        self.block_header(block_ref)
            .map(|header| serialize(&header.raw))
    }

    fn block_header(&self, block_ref: BlockRef) -> Option<IndexedBlockHeader> {
        self.resolve_hash(block_ref).and_then(|block_hash| {
            self.get(Key::BlockHeader(block_hash.clone()))
                .and_then(Value::as_block_header)
                .map(|header| IndexedBlockHeader::new(block_hash, header))
        })
    }
}

impl<T> BlockProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn block_number(&self, hash: &H256) -> Option<u32> {
        self.get(Key::BlockNumber(hash.clone()))
            .and_then(Value::as_block_number)
    }

    fn block_hash(&self, number: u32) -> Option<H256> {
        self.get(Key::BlockHash(number))
            .and_then(Value::as_block_hash)
    }

    fn block(&self, block_ref: BlockRef) -> Option<IndexedBlock> {
        self.resolve_hash(block_ref).and_then(|block_hash| {
            self.block_header(block_hash.clone().into()).map(|header| {
                let transactions = self.block_transactions(block_hash.into());
                IndexedBlock::new(header, transactions)
            })
        })
    }

    fn contains_block(&self, block_ref: BlockRef) -> bool {
        self.resolve_hash(block_ref)
            .and_then(|hash| self.get(Key::BlockHeader(hash)))
            .is_some()
    }

    fn block_transaction_hashes(&self, block_ref: BlockRef) -> Vec<H256> {
        self.resolve_hash(block_ref)
            .and_then(|hash| self.get(Key::BlockTransactions(hash)))
            .and_then(Value::as_block_transactions)
            .map(List::into)
            .unwrap_or_default()
    }

    fn block_transactions(&self, block_ref: BlockRef) -> Vec<IndexedTransaction> {
        self.block_transaction_hashes(block_ref)
            .into_iter()
            .filter_map(|hash| {
                self.get(Key::Transaction(hash))
                    .and_then(Value::as_transaction)
                    .map(|tx| IndexedTransaction::new(hash, tx))
            })
            .collect()
    }
}

impl<T> TransactionMetaProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn transaction_meta(&self, hash: &H256) -> Option<TransactionMeta> {
        self.get(Key::TransactionMeta(hash.clone()))
            .and_then(Value::as_transaction_meta)
    }
}

impl<T> TransactionProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn transaction_bytes(&self, hash: &H256) -> Option<Bytes> {
        self.transaction(hash).map(|tx| serialize(&tx.raw))
    }

    fn transaction(&self, hash: &H256) -> Option<IndexedTransaction> {
        self.get(Key::Transaction(hash.clone()))
            .and_then(Value::as_transaction)
            .map(|tx| IndexedTransaction::new(*hash, tx))
    }
}

impl<T> TransactionOutputProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn transaction_output(
        &self,
        prevout: &OutPoint,
        _transaction_index: usize,
    ) -> Option<TransactionOutput> {
        // return previous transaction outputs only for canon chain transactions
        self.transaction_meta(&prevout.hash)
            .and_then(|_| self.transaction(&prevout.hash))
            .and_then(|tx| tx.raw.outputs.into_iter().nth(prevout.index as usize))
    }

    fn is_spent(&self, prevout: &OutPoint) -> bool {
        self.transaction_meta(&prevout.hash)
            .and_then(|meta| meta.is_spent(prevout.index as usize))
            .unwrap_or(false)
    }
}

impl<T> NullifierTracker for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn contains_nullifier(&self, nullifier: EpochRef) -> bool {
        self.get(Key::Nullifier(nullifier)).is_some()
    }
}

impl<T> TreeStateProvider for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn sprout_tree_at(&self, root: &H256) -> Option<SproutTreeState> {
        self.get(Key::TreeRoot(EpochRef::new(EpochTag::Sprout, *root)))
            .and_then(Value::as_sprout_tree_state)
    }

    fn sapling_tree_at(&self, root: &H256) -> Option<SaplingTreeState> {
        self.get(Key::TreeRoot(EpochRef::new(EpochTag::Sapling, *root)))
            .and_then(Value::as_sapling_tree_state)
    }

    fn sprout_block_root(&self, block_hash: &H256) -> Option<H256> {
        self.get(Key::SproutBlockRoot(*block_hash))
            .and_then(Value::as_sprout_block_root)
    }

    fn sapling_block_root(&self, block_hash: &H256) -> Option<H256> {
        self.block_header(BlockRef::Hash(*block_hash))
            .map(|header| header.raw.final_sapling_root)
    }
}

impl<T> BlockChain for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn insert(&self, block: IndexedBlock) -> Result<(), Error> {
        BlockChainDatabase::insert(self, block)
    }

    fn rollback_best(&self) -> Result<H256, Error> {
        BlockChainDatabase::rollback_best(self)
    }

    fn canonize(&self, block_hash: &H256) -> Result<(), Error> {
        BlockChainDatabase::canonize(self, block_hash)
    }

    fn decanonize(&self) -> Result<H256, Error> {
        BlockChainDatabase::decanonize(self)
    }

    fn block_origin(&self, header: &IndexedBlockHeader) -> Result<BlockOrigin, Error> {
        BlockChainDatabase::block_origin(self, header)
    }
}

impl<T> Forkable for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn fork<'a>(&'a self, side_chain: SideChainOrigin) -> Result<Box<ForkChain + 'a>, Error> {
        BlockChainDatabase::fork(self, side_chain).map(|fork_chain| {
            let boxed: Box<ForkChain> = Box::new(fork_chain);
            boxed
        })
    }

    fn switch_to_fork<'a>(&self, fork: Box<ForkChain + 'a>) -> Result<(), Error> {
        let mut best_block = self.best_block.write();
        *best_block = fork.store().best_block();
        fork.flush()
    }
}

impl<T> CanonStore for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn as_store(&self) -> &Store {
        &*self
    }
}

impl<T> Store for BlockChainDatabase<T>
where
    T: KeyValueDatabase,
{
    fn best_block(&self) -> BestBlock {
        BlockChainDatabase::best_block(self)
    }

    /// get best header
    fn best_header(&self) -> IndexedBlockHeader {
        self.block_header(self.best_block().hash.into())
            .expect("best block header should be in db; qed")
    }
}
