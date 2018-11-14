use ser::Serializable;
use storage::{TransactionMetaProvider, TransactionOutputProvider};
use network::{ConsensusParams};
use script::{Script, verify_script, VerificationFlags, TransactionSignatureChecker, TransactionInputSigner};
use duplex_store::DuplexTransactionOutputProvider;
use deployments::BlockDeployments;
use sigops::transaction_sigops;
use canon::CanonTransaction;
use constants::{COINBASE_MATURITY};
use error::TransactionError;
use VerificationLevel;

pub struct TransactionAcceptor<'a> {
	pub size: TransactionSize<'a>,
	pub bip30: TransactionBip30<'a>,
	pub missing_inputs: TransactionMissingInputs<'a>,
	pub maturity: TransactionMaturity<'a>,
	pub overspent: TransactionOverspent<'a>,
	pub double_spent: TransactionDoubleSpend<'a>,
	pub eval: TransactionEval<'a>,
}

impl<'a> TransactionAcceptor<'a> {
	pub fn new(
		// in case of block validation, it's only current block,
		meta_store: &'a TransactionMetaProvider,
		// previous transaction outputs
		// in case of block validation, that's database and currently processed block
		output_store: DuplexTransactionOutputProvider<'a>,
		consensus: &'a ConsensusParams,
		transaction: CanonTransaction<'a>,
		verification_level: VerificationLevel,
		height: u32,
		time: u32,
		transaction_index: usize,
		deployments: &'a BlockDeployments<'a>,
	) -> Self {
		/*
			TODO:
			Sprout:
				reject transactions which are intended for Overwinter and beyond
			Overwinter:
				tx version
				tx version group
				reject transactions with valid version but missing overwinter flag
				reject transactions intended for Sprout
				check that all transactions are unexpired
			Sapling:
				tx version
				tx version group
				reject transactions intended for Sprout
				check that all transactions are unexpired
				block max size changes!!!
		*/


		trace!(target: "verification", "Tx verification {}", transaction.hash.to_reversed_str());
		TransactionAcceptor {
			size: TransactionSize::new(transaction, consensus, height),
			bip30: TransactionBip30::new_for_sync(transaction, meta_store),
			missing_inputs: TransactionMissingInputs::new(transaction, output_store, transaction_index),
			maturity: TransactionMaturity::new(transaction, meta_store, height),
			overspent: TransactionOverspent::new(transaction, output_store),
			double_spent: TransactionDoubleSpend::new(transaction, output_store),
			eval: TransactionEval::new(transaction, output_store, consensus, verification_level, height, time, deployments),
		}
	}

	pub fn check(&self) -> Result<(), TransactionError> {
		self.size.check()?;
		self.bip30.check()?;
		self.missing_inputs.check()?;
		self.maturity.check()?;
		self.overspent.check()?;
		self.double_spent.check()?;
		self.eval.check()?;
		Ok(())
	}
}

pub struct MemoryPoolTransactionAcceptor<'a> {
	pub missing_inputs: TransactionMissingInputs<'a>,
	pub maturity: TransactionMaturity<'a>,
	pub overspent: TransactionOverspent<'a>,
	pub sigops: TransactionSigops<'a>,
	pub double_spent: TransactionDoubleSpend<'a>,
	pub eval: TransactionEval<'a>,
}

impl<'a> MemoryPoolTransactionAcceptor<'a> {
	pub fn new(
		// TODO: in case of memory pool it should be db and memory pool
		meta_store: &'a TransactionMetaProvider,
		// in case of memory pool it should be db and memory pool
		output_store: DuplexTransactionOutputProvider<'a>,
		consensus: &'a ConsensusParams,
		transaction: CanonTransaction<'a>,
		height: u32,
		time: u32,
		deployments: &'a BlockDeployments<'a>,
	) -> Self {
		trace!(target: "verification", "Mempool-Tx verification {}", transaction.hash.to_reversed_str());
		let transaction_index = 0;
		let max_block_sigops = consensus.max_block_sigops();
		MemoryPoolTransactionAcceptor {
			missing_inputs: TransactionMissingInputs::new(transaction, output_store, transaction_index),
			maturity: TransactionMaturity::new(transaction, meta_store, height),
			overspent: TransactionOverspent::new(transaction, output_store),
			sigops: TransactionSigops::new(transaction, output_store, consensus, max_block_sigops, time),
			double_spent: TransactionDoubleSpend::new(transaction, output_store),
			eval: TransactionEval::new(transaction, output_store, consensus, VerificationLevel::Full, height, time, deployments),
		}
	}

	pub fn check(&self) -> Result<(), TransactionError> {
		// Bip30 is not checked because we don't need to allow tx pool acceptance of an unspent duplicate.
		// Tx pool validation is not strinctly a matter of consensus.
		try!(self.missing_inputs.check());
		try!(self.maturity.check());
		try!(self.overspent.check());
		try!(self.sigops.check());
		try!(self.double_spent.check());
		try!(self.eval.check());
		Ok(())
	}
}

/// Bip30 validation
///
/// A transaction hash that exists in the chain is not acceptable even if
/// the original is spent in the new block. This is not necessary nor is it
/// described by BIP30, but it is in the code referenced by BIP30. As such
/// the tx pool need only test against the chain, skipping the pool.
///
/// source:
/// https://github.com/libbitcoin/libbitcoin/blob/61759b2fd66041bcdbc124b2f04ed5ddc20c7312/src/chain/transaction.cpp#L780-L785
pub struct TransactionBip30<'a> {
	transaction: CanonTransaction<'a>,
	store: &'a TransactionMetaProvider,
}

impl<'a> TransactionBip30<'a> {
	fn new_for_sync(
		transaction: CanonTransaction<'a>,
		store: &'a TransactionMetaProvider,
	) -> Self {
		TransactionBip30 {
			transaction: transaction,
			store: store,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		match self.store.transaction_meta(&self.transaction.hash) {
			Some(ref meta) if !meta.is_fully_spent() => {
				Err(TransactionError::UnspentTransactionWithTheSameHash)
			},
			_ => Ok(())
		}
	}
}

pub struct TransactionMissingInputs<'a> {
	transaction: CanonTransaction<'a>,
	store: DuplexTransactionOutputProvider<'a>,
	transaction_index: usize,
}

impl<'a> TransactionMissingInputs<'a> {
	fn new(transaction: CanonTransaction<'a>, store: DuplexTransactionOutputProvider<'a>, transaction_index: usize) -> Self {
		TransactionMissingInputs {
			transaction: transaction,
			store: store,
			transaction_index: transaction_index,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		let missing_index = self.transaction.raw.inputs.iter()
			.position(|input| {
				let is_not_null = !input.previous_output.is_null();
				let is_missing = self.store.transaction_output(&input.previous_output, self.transaction_index).is_none();
				is_not_null && is_missing
			});

		match missing_index {
			Some(index) => Err(TransactionError::Input(index)),
			None => Ok(())
		}
	}
}

pub struct TransactionMaturity<'a> {
	transaction: CanonTransaction<'a>,
	store: &'a TransactionMetaProvider,
	height: u32,
}

impl<'a> TransactionMaturity<'a> {
	fn new(transaction: CanonTransaction<'a>, store: &'a TransactionMetaProvider, height: u32) -> Self {
		TransactionMaturity {
			transaction: transaction,
			store: store,
			height: height,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		// TODO: this is should also fail when we are trying to spend current block coinbase
		let immature_spend = self.transaction.raw.inputs.iter()
			.any(|input| match self.store.transaction_meta(&input.previous_output.hash) {
				Some(ref meta) if meta.is_coinbase() && self.height < meta.height() + COINBASE_MATURITY => true,
				_ => false,
			});

		if immature_spend {
			Err(TransactionError::Maturity)
		} else {
			Ok(())
		}
	}
}

pub struct TransactionOverspent<'a> {
	transaction: CanonTransaction<'a>,
	store: DuplexTransactionOutputProvider<'a>,
}

impl<'a> TransactionOverspent<'a> {
	fn new(transaction: CanonTransaction<'a>, store: DuplexTransactionOutputProvider<'a>) -> Self {
		TransactionOverspent {
			transaction: transaction,
			store: store,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		if self.transaction.raw.is_coinbase() {
			return Ok(());
		}

		let available = self.transaction.raw.inputs.iter()
			.map(|input| self.store.transaction_output(&input.previous_output, usize::max_value()).map(|o| o.value).unwrap_or(0))
			.sum::<u64>();

		let spends = self.transaction.raw.total_spends();

		if spends > available {
			Err(TransactionError::Overspend)
		} else {
			Ok(())
		}
	}
}

pub struct TransactionSigops<'a> {
	transaction: CanonTransaction<'a>,
	store: DuplexTransactionOutputProvider<'a>,
	consensus_params: &'a ConsensusParams,
	max_sigops: usize,
	time: u32,
}

impl<'a> TransactionSigops<'a> {
	fn new(transaction: CanonTransaction<'a>, store: DuplexTransactionOutputProvider<'a>, consensus_params: &'a ConsensusParams, max_sigops: usize, time: u32) -> Self {
		TransactionSigops {
			transaction: transaction,
			store: store,
			consensus_params: consensus_params,
			max_sigops: max_sigops,
			time: time,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		let bip16_active = self.time >= self.consensus_params.bip16_time;
		let checkdatasig_active = false;
		let sigops = transaction_sigops(&self.transaction.raw, &self.store, bip16_active, checkdatasig_active);
		if sigops > self.max_sigops {
			Err(TransactionError::MaxSigops)
		} else {
			Ok(())
		}
	}
}

pub struct TransactionEval<'a> {
	transaction: CanonTransaction<'a>,
	store: DuplexTransactionOutputProvider<'a>,
	verification_level: VerificationLevel,
	verify_p2sh: bool,
	verify_strictenc: bool,
	verify_locktime: bool,
	verify_checksequence: bool,
	verify_dersig: bool,
	verify_nulldummy: bool,
	verify_monolith_opcodes: bool,
	verify_magnetic_anomaly_opcodes: bool,
	verify_sigpushonly: bool,
	verify_cleanstack: bool,
}

impl<'a> TransactionEval<'a> {
	fn new(
		transaction: CanonTransaction<'a>,
		store: DuplexTransactionOutputProvider<'a>,
		params: &ConsensusParams,
		verification_level: VerificationLevel,
		height: u32,
		time: u32,
		deployments: &'a BlockDeployments,
	) -> Self {
		let verify_p2sh = time >= params.bip16_time;
		let verify_strictenc = false;
		let verify_locktime = height >= params.bip65_height;
		let verify_dersig = height >= params.bip66_height;
		let verify_monolith_opcodes = false;
		let verify_magnetic_anomaly_opcodes = false;

		let verify_checksequence = deployments.csv();
		let verify_sigpushonly = verify_magnetic_anomaly_opcodes;
		let verify_cleanstack = verify_magnetic_anomaly_opcodes;

		TransactionEval {
			transaction: transaction,
			store: store,
			verification_level: verification_level,
			verify_p2sh: verify_p2sh,
			verify_strictenc: verify_strictenc,
			verify_locktime: verify_locktime,
			verify_checksequence: verify_checksequence,
			verify_dersig: verify_dersig,
			verify_nulldummy: false,
			verify_monolith_opcodes: verify_monolith_opcodes,
			verify_magnetic_anomaly_opcodes: verify_magnetic_anomaly_opcodes,
			verify_sigpushonly: verify_sigpushonly,
			verify_cleanstack: verify_cleanstack,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		if self.verification_level == VerificationLevel::Header
			|| self.verification_level == VerificationLevel::NoVerification {
			return Ok(());
		}

		if self.transaction.raw.is_coinbase() {
			return Ok(());
		}

		let signer: TransactionInputSigner = self.transaction.raw.clone().into();

		let mut checker = TransactionSignatureChecker {
			signer: signer,
			input_index: 0,
			input_amount: 0,
		};

		for (index, input) in self.transaction.raw.inputs.iter().enumerate() {
			let output = self.store.transaction_output(&input.previous_output, usize::max_value())
				.ok_or_else(|| TransactionError::UnknownReference(input.previous_output.hash.clone()))?;

			checker.input_index = index;
			checker.input_amount = output.value;

			let input: Script = input.script_sig.clone().into();
			let output: Script = output.script_pubkey.into();

			let flags = VerificationFlags::default()
				.verify_p2sh(self.verify_p2sh)
				.verify_strictenc(self.verify_strictenc)
				.verify_locktime(self.verify_locktime)
				.verify_checksequence(self.verify_checksequence)
				.verify_dersig(self.verify_dersig)
				.verify_nulldummy(self.verify_nulldummy)
				.verify_concat(self.verify_monolith_opcodes)
				.verify_split(self.verify_monolith_opcodes)
				.verify_and(self.verify_monolith_opcodes)
				.verify_or(self.verify_monolith_opcodes)
				.verify_xor(self.verify_monolith_opcodes)
				.verify_div(self.verify_monolith_opcodes)
				.verify_mod(self.verify_monolith_opcodes)
				.verify_bin2num(self.verify_monolith_opcodes)
				.verify_num2bin(self.verify_monolith_opcodes)
				.verify_checkdatasig(self.verify_magnetic_anomaly_opcodes)
				.verify_sigpushonly(self.verify_sigpushonly)
				.verify_cleanstack(self.verify_cleanstack);

			try!(verify_script(&input, &output, &flags, &checker)
				.map_err(|e| TransactionError::Signature(index, e)));
		}

		Ok(())
	}
}

pub struct TransactionDoubleSpend<'a> {
	transaction: CanonTransaction<'a>,
	store: DuplexTransactionOutputProvider<'a>,
}

impl<'a> TransactionDoubleSpend<'a> {
	fn new(transaction: CanonTransaction<'a>, store: DuplexTransactionOutputProvider<'a>) -> Self {
		TransactionDoubleSpend {
			transaction: transaction,
			store: store,
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		for input in &self.transaction.raw.inputs {
			if self.store.is_spent(&input.previous_output) {
				return Err(TransactionError::UsingSpentOutput(
					input.previous_output.hash.clone(),
					input.previous_output.index
				))
			}
		}
		Ok(())
	}
}

/// The encoded size of the transaction MUST be less than or equal to current max limit.
pub struct TransactionSize<'a> {
	transaction: CanonTransaction<'a>,
	max_size: usize,
}

impl<'a> TransactionSize<'a> {
	fn new(transaction: CanonTransaction<'a>, consensus: &'a ConsensusParams, height: u32) -> Self {
		TransactionSize {
			transaction: transaction,
			max_size: consensus.max_transaction_size(height),
		}
	}

	fn check(&self) -> Result<(), TransactionError> {
		let size = self.transaction.raw.serialized_size();
		if size > self.max_size {
			Err(TransactionError::MaxSize)
		} else {
			Ok(())
		}
	}
}
