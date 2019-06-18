# JSON-RPC

The JSON-RPC interface is served on port :8232 for mainnet and :18232 for testnet unless you specified otherwise. So if you are using testnet, you will need to change the port in the sample curl requests shown below.

### Network

The Zebra `network` interface.

#### addnode

Add the node.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "addnode", "params": ["127.0.0.1:8233", "add"], "id":1 }' localhost:8232

Remove the node.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "addnode", "params": ["127.0.0.1:8233", "remove"], "id":1 }' localhost:8232

Connect to the node.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "addnode", "params": ["127.0.0.1:8233", "onetry"], "id":1 }' localhost:8232

#### getaddednodeinfo

Query info for all added nodes.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getaddednodeinfo", "params": [true] }' localhost:8232

Query info for the specified node.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getaddednodeinfo", "params": [true, "192.168.0.201"] }' localhost:8232

#### getconnectioncount

Get the peer count.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getconnectioncount", "params": [] }' localhost:8232

### Blockchain

The Zebra `blockchain` data interface.

#### getbestblockhash

Get hash of best block.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getbestblockhash", "params": [], "id":1 }' localhost:8232

#### getblockcount

Get height of best block.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id":1 }' localhost:8232

#### getblockhash

Get hash of block at given height.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getblockhash", "params": [0], "id":1 }' localhost:8232

#### getdifficulty

Get proof-of-work difficulty as a multiple of the minimum difficulty

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getdifficulty", "params": [], "id":1 }' localhost:8232

#### getblock

Get information on given block.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getblock", "params": ["0002a26c902619fc964443264feb16f1e3e2d71322fc53dcb81cc5d797e273ed"], "id":1 }' localhost:8232

#### gettxout

Get details about an unspent transaction output.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "gettxout", "params": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", 0], "id":1 }' localhost:8232

#### gettxoutsetinfo

Get statistics about the unspent transaction output set.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "gettxoutsetinfo", "params": [], "id":1 }' localhost:8232

### Miner

The Zebra `miner` data interface.

#### getblocktemplate

Get block template for mining.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getblocktemplate", "params": [{"capabilities": ["coinbasetxn", "workid", "coinbase/append"]}], "id":1 }' localhost:8232

### Raw

The Zebra `raw` data interface.


#### getrawtransaction

Return the raw transaction data.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "getrawtransaction", "params": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"], "id":1 }' localhost:8232

#### decoderawtransaction

Return an object representing the serialized, hex-encoded transaction.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "decoderawtransaction", "params": ["01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"], "id":1 }' localhost:8232

#### createrawtransaction

Create a transaction spending the given inputs and creating new outputs.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "createrawtransaction", "params": [[{"txid":"4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b","vout":0}],{"t1h8SqgtM3QM5e2M8EzhhT1yL2PXXtA6oqe":0.01}], "id":1 }' localhost:8232

#### sendrawtransaction

Adds transaction to the memory pool && relays it to the peers.

    curl -H 'content-type: application/json' --data-binary '{"jsonrpc": "2.0", "method": "sendrawtransaction", "params": ["01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"], "id":1 }' localhost:8232