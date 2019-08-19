# Zebra Anti-Design Document

This document describes current features of the Zebra codebase (as received from
Parity) which we don't want to keep in future versions of Zebra.

## Dependencies

Dependencies we should not use:

* `bigint`: using a general-purpose bigint library in cryptography is a big code
  smell.

## `zebra-primitives`

This module contains three main pieces of functionality, all of which should
disappear:

### The `Bytes` type

Defined in `src/bytes.rs`, this provides a newtype wrapper around a `Vec<u8>`.
Using a newtype wrapper means that all of the existing methods on `Vec<T>` are
inaccessible to users of the `Bytes` type.  Some of them are reimplemented, but
with different names: for instance, `Vec::default()` becomes `Bytes::new`.
Others have similar functionality but different names, such as
`Bytes::new_with_len` instead of `Vec::with_capacity`.  
In addition to adding cognitive overhead, using `Bytes` everywhere instead of
the normal idiom of `Vec<T>` / `&[T]` encourages the rest of the API not to
consider ownership in function arguments, and prevents the use of generic
functions.

### The fixed-size hash types

Defined in `src/hash.rs`, these provide fixed-size containers for 32, 48, 96,
160, 256, 264, 512, and 520-bit values.  It seems that these were originally
intended to be used for hash outputs, but then morphed into a general-purpose
"bag of bits" structure.  Where it is needed to represent a hash output or other
fixed-size bit structure, a newtype with *semantic meaning* should be used,
e.g., `Sha256Output` instead of `H256` or `TruncatedDoubleSha256` instead of
`H32`, so that the newtype wrapper actually *adds information*.

### The `Compact` type

This type has no documentation on its intended purpose; it appears to be used
to represent block difficulties, in which case it should be part of the
block-handling code and have a name that reflects its intended purpose, rather
than appearing to be general-purpose.

Note: this type is distinct from the `CompactSize` type, which is unrelated and
defined elsewhere.

## `zebra-serialization` and `zebra-serialization_derive`

These crates should be replaced entirely.

The current Zebra code uses a "magic" derive implementation to convert struct
definitions into consensus-critical (!) network serialization implementations,
in a way that "just happens" to work.  For instance, a `usize` is encoded as a
`CompactSize` and a `Vec<T>` is encoded as a `CompactSize` length followed by
the `T`s, so that a `BlockTransactionsRequest` defined as

```rust
// zebra-message/src/common/block_transactions_request.rs
#[derive(Debug, PartialEq)]
pub struct BlockTransactionsRequest {
    pub blockhash: H256,
    pub indexes: Vec<usize>,
}
```

has a derived (de)serialization implementation that coincidentally matches the
specification in the BIP that defines that message.  (As a side note, that BIP
isn't used in Zcash anyways, so we don't even need it at all).

Instead, consensus-critical wire formats should be specified precisely in code
we can read in one place (say, the implementation of the relevant message type).

Finally, the serialization and deserialization traits are badly specified, so
they are not defined over generic `Read` and `Write` implementations, but only
on the custom `Bytes` type.  In addition to making the code less clean, this
causes a performance loss because it forces extra memory copies.

A worse example of the same fundamental problem occurs with the block message
structure, defined as
```rust
// zebra-message/src/types/block.rs

use zebra_chain::Block as ChainBlock;

#[derive(Debug, PartialEq)]
pub struct Block {
    pub block: ChainBlock,
}
```

Nothing distinguishes the `Block` message structure from the inner
`zebra_chain::Block`, and the inner struct is not encapsulated.  The
actual (de)serialization implementation for the message uses the
custom derives on the inner `zebra_chain::Block` struct, defined by

```rust
// zebra-chain/src/block.rs

#[derive(Debug, PartialEq, Clone, Serializable, Deserializable)]
pub struct Block {
    pub block_header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

// zebra-chain/src/block_header.rs

#[derive(PartialEq, Clone, Serializable, Deserializable)]
pub struct BlockHeader {
    pub version: u32,
    pub previous_header_hash: H256,
    pub merkle_root_hash: H256,
    pub final_sapling_root: H256,
    pub time: u32,
    pub bits: Compact,
    pub nonce: H256,
    pub solution: EquihashSolution,
}
```

In other words, the consensus-critical block message encoding is
derived from the source ordering of the fields on the internal `Block`
and `BlockHeader` structs, so that changing or reordering fields
on the internal representation of the block breaks the network
protocol.

