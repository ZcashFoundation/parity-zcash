#!/usr/bin/env bash

./zebrad --jsonrpc-port ${PORT:-8080} --testnet --data-dir=./.zebra-testnet
