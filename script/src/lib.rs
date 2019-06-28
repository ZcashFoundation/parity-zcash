extern crate bitcrypto as crypto;
extern crate byteorder;
extern crate chain;
extern crate keys;
extern crate log;
extern crate primitives;
extern crate serialization as ser;

#[cfg(test)]
extern crate rustc_hex as hex;
#[cfg(test)]
extern crate serde_json;

mod builder;
mod error;
mod flags;
mod interpreter;
mod num;
mod opcode;
mod script;
mod sign;
mod stack;
mod verify;

pub use primitives::{bytes, hash};

pub use self::builder::Builder;
pub use self::error::Error;
pub use self::flags::VerificationFlags;
pub use self::interpreter::{eval_script, verify_script};
pub use self::num::Num;
pub use self::opcode::Opcode;
pub use self::script::{Script, ScriptAddress, ScriptType};
pub use self::sign::{SighashBase, SighashCache, TransactionInputSigner, UnsignedTransactionInput};
pub use self::stack::Stack;
pub use self::verify::{NoopSignatureChecker, SignatureChecker, TransactionSignatureChecker};
