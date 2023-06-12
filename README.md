## Components
- Go Ethereum: The modified Geth client
- OpenSGX: The emulated SGX platform and mempool implementation
- geth-runner: A rust client for initializing and managing local consensus networks.
- WAN-test: Python scripts for WAN tests.

## How to Run
- Edit *config.toml* to configure a consensus network
	```
	...
	[test]
	test = false # If reset, other variables in this catagory are ignored, and no transactions will be sent.
	n = 100 # The number of transactions to be sent to EACH node.
	period = 35 # How long the network runs
	start_nonce = 0 # Should be set with the number of transactions committed
	payload = 0 # The length of the payload fied of each transaction

	[node]
	...
	count = 20 # Total number of nodes
	sealer_count = 16 # Total number of sealers
	random_connect = true # If set, the peers of each node are randomly chosen and connected. Field "peer_count" should be specified. If reset, the peers of each node should be specified by field "connection".
	peer_count = 3

	[init]
	...
	mining_period = 5 # Block interval (second)

	[run]
	wan = true # If set, nodes are run in different hosts, specified in field hosts. If reset, all nodes are run in the first host specified in field hosts.
	...
	```
	- Initialize nodes
		- `cd geth-runner; cargo run -- --init`
	- Pre-run nodes locally
		- Reset `test.test` in *config.toml*. Set `period` to at least 10 seconds.
		- Execute `cargo run -- --run` TWICE, confirming that nodes can propose blocks normally. The reason for running twice is relevant to a deadlock problem in Clique.
	- Set `test.test` in *config.toml*. Set variables in `test` to configure the number of transactions to be sent from EACH node, how long the network runs, and the size of transaction payloads.
	- Now we are ready to establish a consensus network with initialized nodes.
		- For a local network:
			- `cargo run -- --run`
		- For a network with different hosts:
			- `cd ../WAN-test`
			- For the first run, execute `python run.py --init`. The script uploads all required binaries and configuration files and establish the consensus network. For next runs, execute `python run.py`.
