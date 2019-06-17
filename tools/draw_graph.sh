#!/bin/bash

cargo install cargo-deps

# And draw dependencies graph using cargo deps
cargo deps > tools/graph.dot

# Let's fix graph ratio
patch tools/graph.dot tools/graph_ratio.diff

# Requires graphviz. If on macOS: 'brew install graphviz'
dot -Tsvg > tools/graph.svg tools/graph.dot
