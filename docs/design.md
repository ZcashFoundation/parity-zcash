Zebra Design Document
=====================

This document sketches the future design for Zebra.  It is written independently
of the current design (as instantiated in the Parity code): some parts may make
the same design decisions, and other parts may make different design decisions,
but in any case the decision is independent of the current state of the Zebra
source code.

Having an indepenent design means that there are two paths available: we could
either implement it from scratch, or we could incrementally refactor the current
Zebra design towards the one in this document.

Desiderata
==========

The following are general desiderata for Zebra:

* [George's list..]

* As much as reasonably possible, it and its dependencies should be
  implemented in Rust.  While it may not make sense to require this in
  every case (for instance, it probably doesn't make sense to rewrite
  libsecp256k1 in Rust, instead of using the same upstream library as
  Bitcoin), we should generally aim for it.

* As much as reasonably possible, Zebra should minimize trust in
  required dependencies.  Note that "minimize number of dependencies"
  is usually a proxy for this desiderata, but is not exactly the same:
  for instance, a collection of crates like the tokio crates are all
  developed together and have one trust boundary.
  
* Zebra should be well-factored internally into a collection of
  component libraries which can be used by other applications to
  perform Zcash-related tasks.  Implementation details of each
  component should not leak into all other components, as is currently
  the case.
  
* Zebra should checkpoint on Sapling activation and drop all
  Sprout-related functionality not required post-Sapling.
  
Internal Structure
==================

The following is a list of internal component libraries (crates), and
a description of functional responsibility.  In this document, they
are currently named `zebra2-foo` to avoid naming collisions or false
cognates with existing zebra crates, and (as noted at the beginning)
their design is independent and specified only by this document.

`zebra2-chain`
--------------

### Internal Dependencies

None: this the core data structure definitions.

### Responsible for

- definitions of commonly used data structures, e.g.,
  - `CompactSize`,
  - `Address`,
  - `KeyPair`...
- blockchain data structures: blocks, transactions, etc.

### Exported types

- [...]

`zebra2-network`
----------------

### Internal Dependencies

- `zebra2-chain`

### Responsible for

- p2p networking code
- giant enum with definitions of all network messages
- turns bytes into validated internal reprs and vice versa

### Exported types

- Some kind of `Message` enum ???
- [...]

`zebra2-storage`
----------------

### Internal Dependencies

- `zebra2-chain` for data structure definitions.

### Responsible for

- block storage API
  - operates on raw bytes for blocks
  - primarily aimed at network replication
  - can be used to rebuild the database below
- maintaining a database of tx, address, etc data
  - this database can be blown away and rebuilt from the blocks, which are
    otherwise unused.
  - threadsafe, typed lookup API that completely encapsulates the database logic
  - handles stuff like "transactions are reference counted by outputs" etc.
- maintaining a database for wallet state
  - ??? what does this entail exactly

### Exported types

- [...]

`zebra2-script`
---------------

### Internal Dependencies

- ??? depends on how it's implemented internally

### Responsible for

- the minimal Bitcoin script implementation required for Zcash

### Notes

This can wrap an existing script implementation at the beginning.

If this existed in a "good" way, we could use it to implement tooling
for Zcash script inspection, debugging, etc.

### Questions

- How does this interact with NU4 script changes?

### Exported types

- [...]

`zebra2-consensus`
------------------

### Internal Dependencies

- `zebra2-chain`
- `zebra2-storage`
- `zebra2-script`

### Responsible for

- consensus-specific parameters (network magics, genesis block, pow
  parameters, etc) that determine the network consensus
- validation logic to decide whether blocks are acceptable 
- consensus logic to decide which block is the current block

### Exported types

- [...]

`zebra2-rpc`
------------

### Internal Dependencies

- `zebra2-chain` for data structure definitions
- `zebra2-network` possibly? for definitions of network messages?

### Responsible for

- rpc interface

### Exported types

- [...]

`zebra2-client`
-----------------

### Internal Dependencies

- `zebra2-chain` for structure definitions
- `zebra2-storage` for transaction queries and client/wallet state storage
- `zebra2-script` possibly? for constructing transactions

### Responsible for

- implementation of some event a user might trigger
- would be used to implement a wallet
- create transactions, monitors shielded wallet state, etc.

### Notes 

Hopefully this can be backed by @str4d's light wallet code.

Hopefully the interface could be designed to make it easy to implement
as a process-separated softHSM either now or later, depending on
delta-work.

### Exported types

- [...]

`zebra2-reactor`
----------------

### Internal Dependencies

- `zebra2-chain`
- `zebra2-network`
- `zebra2-storage`
- `zebra2-consensus`
- `zebra2-client`
- `zebra2-rpc`

### Responsible for

- actually running the server
- connecting functionality in dependencies

### Exported types

- [...]

`zebrad2`
---------

Abscissa-based application which loads configs, starts the reactor.

Unassigned functionality
------------------------

Responsibility for this functionality needs to be assigned to one of
the modules above (subject to discussion):

- [ ... add to this list ... ]
