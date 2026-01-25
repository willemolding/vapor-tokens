# Usage

## Prerequisites

- Anchor 
- Rust

## Deploying a new Vapor Token

Set the following env vars or use a .env file. Use your own name, symbol, and metadata URI or copy these

```shell
TOKEN_NAME="My Vapor Token"
TOKEN_SYMBOL=VAPOR
TOKEN_URI=https://blush-wonderful-pinniped-843.mypinata.cloud/ipfs/bafkreicol3bfrmwndtpzqati3p2pgpe42akaxzb5rk367247iynszmf2v4
MINT_RECIPIENT=AcXfMtHLTqmv5cogRFEwbJBfKMuvXnC3VGatfYRKHH1k
TOKEN_SUPPLY=1000000
TOKEN_DECIMALS=9
```

The mint recipient is the account that will receive the entire supply at creation time.

Then run

```shell
cd anchor
anchor migrate
```

Copy the Mint (token address) value for use later

## Using the CLI Wallet

Set the following env vars, or create a `.env` file

```shell
SOL_RPC=https://api.devnet.solana.com
MINT=<your-token-mint>
```

Create a new vapor address with

```shell
cd crates/cli-wallet
cargo run -- gen-address <destination-solana-address> 
```

> [!IMPORTANT]
> Use your Solana address here, not the associated token address

> [!IMPORTANT]
> Be sure to delete any old wallet file () used with a different token

Sync the wallet and check addresses with


```shell
cargo run -- list
```

Condense a vapor address deposit to its destination with

```shell
cargo run -- condense <vapor-address> --keypair ~/.config/solana/id.json 
```

> [!NOTE]
> Ensure the keypair has devnet SOL to pay fees

## Development

### Prerequisites

### Updating the Circuits

After making any changes to the circuits you must perform another trusted setup and regenerated the on-chain verifier and docker prover image. 

```shell
just trusted_setup
just build_verifier_program
just build_docker
```

> [!IMPORTANT]
> The trusted setup uses randomization internally so even with no code changes running it again will generate new prove and verify keys.
> To ensure the contracts and rust crates stay in sync run the trusted setup once and then build the docker container and verifier program.

