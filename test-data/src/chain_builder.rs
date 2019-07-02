use ser::Serializable;
use zebra_chain::{
    IndexedTransaction, JoinSplit, OutPoint, Sapling, Transaction, TransactionInput,
    TransactionOutput,
};
use zebra_primitives::bytes::Bytes;
use zebra_primitives::hash::H256;

#[derive(Debug, Default, Clone)]
pub struct ChainBuilder {
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Default, Clone)]
pub struct TransactionBuilder {
    pub transaction: Transaction,
}

impl ChainBuilder {
    pub fn new() -> ChainBuilder {
        ChainBuilder {
            transactions: Vec::new(),
        }
    }

    pub fn at(&self, transaction_index: usize) -> Transaction {
        self.transactions[transaction_index].clone()
    }

    pub fn hash(&self, transaction_index: usize) -> H256 {
        self.transactions[transaction_index].hash()
    }

    pub fn size(&self, transaction_index: usize) -> usize {
        self.transactions[transaction_index].serialized_size()
    }
}

impl Into<Transaction> for TransactionBuilder {
    fn into(self) -> Transaction {
        self.transaction
    }
}

impl Into<IndexedTransaction> for TransactionBuilder {
    fn into(self) -> IndexedTransaction {
        IndexedTransaction {
            hash: self.transaction.hash(),
            raw: self.transaction,
        }
    }
}

impl TransactionBuilder {
    pub fn overwintered() -> TransactionBuilder {
        let mut builder = TransactionBuilder::default();
        builder.transaction.overwintered = true;
        builder
    }

    pub fn coinbase() -> TransactionBuilder {
        let mut builder = TransactionBuilder::default();
        builder
            .transaction
            .inputs
            .push(TransactionInput::coinbase(Default::default()));
        builder
    }

    pub fn with_version(version: i32) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.set_version(version)
    }

    pub fn with_output(value: u64) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.add_output(value)
    }

    pub fn with_default_input(output_index: u32) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.add_input(&Transaction::default(), output_index)
    }

    pub fn with_sapling(sapling: Sapling) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.set_sapling(sapling)
    }

    pub fn with_join_split(join_split: JoinSplit) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.set_join_split(join_split)
    }

    pub fn with_input(transaction: &Transaction, output_index: u32) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.add_input(transaction, output_index)
    }

    pub fn reset(self) -> TransactionBuilder {
        TransactionBuilder::default()
    }

    pub fn into_input(self, output_index: u32) -> TransactionBuilder {
        let builder = TransactionBuilder::default();
        builder.add_input(&self.transaction, output_index)
    }

    pub fn set_overwintered(mut self, overwintered: bool) -> TransactionBuilder {
        self.transaction.overwintered = overwintered;
        self
    }

    pub fn set_version(mut self, version: i32) -> TransactionBuilder {
        self.transaction.version = version;
        self
    }

    pub fn set_version_group_id(mut self, version_group_id: u32) -> TransactionBuilder {
        self.transaction.version_group_id = version_group_id;
        self
    }

    pub fn add_output(mut self, value: u64) -> TransactionBuilder {
        self.transaction.outputs.push(TransactionOutput {
            value: value,
            script_pubkey: Bytes::new_with_len(0),
        });
        self
    }

    pub fn set_output(mut self, value: u64) -> TransactionBuilder {
        self.transaction.outputs = vec![TransactionOutput {
            value: value,
            script_pubkey: Bytes::new_with_len(0),
        }];
        self
    }

    pub fn add_default_input(self, output_index: u32) -> TransactionBuilder {
        self.add_input(&Transaction::default(), output_index)
    }

    pub fn add_input(mut self, transaction: &Transaction, output_index: u32) -> TransactionBuilder {
        self.transaction.inputs.push(TransactionInput {
            previous_output: OutPoint {
                hash: transaction.hash(),
                index: output_index,
            },
            script_sig: Bytes::new_with_len(0),
            sequence: 0xffffffff,
        });
        self
    }

    pub fn set_default_input(self, output_index: u32) -> TransactionBuilder {
        self.set_input(&Transaction::default(), output_index)
    }

    pub fn set_input(mut self, transaction: &Transaction, output_index: u32) -> TransactionBuilder {
        self.transaction.inputs = vec![TransactionInput {
            previous_output: OutPoint {
                hash: transaction.hash(),
                index: output_index,
            },
            script_sig: Bytes::new_with_len(0),
            sequence: 0xffffffff,
        }];
        self
    }

    pub fn set_sapling(mut self, sapling: Sapling) -> TransactionBuilder {
        self.transaction.sapling = Some(sapling);
        self
    }

    pub fn set_join_split(mut self, join_split: JoinSplit) -> TransactionBuilder {
        self.transaction.join_split = Some(join_split);
        self
    }

    pub fn set_expiry_height(mut self, expiry_height: u32) -> TransactionBuilder {
        self.transaction.expiry_height = expiry_height;
        self
    }

    pub fn lock(mut self) -> Self {
        self.transaction.inputs[0].sequence = 0;
        self.transaction.lock_time = 500000;
        self
    }

    pub fn store(self, chain: &mut ChainBuilder) -> Self {
        chain.transactions.push(self.transaction.clone());
        self
    }

    pub fn hash(self) -> H256 {
        self.transaction.hash()
    }

    pub fn add_default_join_split(mut self) -> Self {
        self.transaction.join_split = Some(Default::default());
        self
    }
}
