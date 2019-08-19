# Zebra Design Document

This document sketches the future design for Zebra.  It is written independently
of the current design (as instantiated in the Parity code): some parts may make
the same design decisions, and other parts may make different design decisions,
but in any case the decision is independent of the current state of the Zebra
source code.

Having an indepenent design means that there are two paths available: we could
either implement it from scratch, or we could incrementally refactor the current
Zebra design towards the one in this document.

## Desiderata

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
  
## Internal Structure

The following is a list of internal component libraries (crates), and
a description of functional responsibility.  In this document, they
are currently named `zebra2-foo` to avoid naming collisions or false
cognates with existing zebra crates, and (as noted at the beginning)
their design is independent and specified only by this document.

### zebra2-primitives

Responsible for:

- definitions of commonly used data structures (e.g., `CompactSize`,
  `Address`, `KeyPair`) and their serialization implementations.

Exported types:

- [...]

### zebra2-chain

Responsible for:

- blockchain data structures: blocks, transactions, etc.

Exported types:

- [...]

### zebra2-storage

Responsible for:

- block storage

Exported types:

- [...]

Questions:

- what's the right abstraction layer for block storage? a filesystem
  or something more cloud-friendly?
  
### zebra2-database

Responsible for:

- indexing blocks whose storage is managed by `zebra2-storage`.

Exported types:

- [...]

### zebra2-script

Responsible for:

- the minimal Bitcoin script implementation required for Zcash

Exported types:

- [...]

### zebra2-consensus

Responsible for:

- consensus-specific parameters (network magics, genesis block, pow
  parameters, etc) that determine the network consensus
- validation logic to decide whether blocks are acceptable 
- consensus logic to decide which block is the current block

Exported types:

- [...]

### zebra2-network

Responsible for:

- p2p networking code
- definitions of all network messages

Exported types:

- [...]

### zebra2-rpc

Responsible for:

- rpc interface

Exported types:

- [...]

### zebrad2

Abscissa-based application combining the previous components.

### Unassigned functionality

Responsibility for this functionality needs to be assigned to one of
the modules above (subject to discussion):

- [ ... add to this list ... ]
