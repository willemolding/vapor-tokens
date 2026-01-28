# <img src="./docs/VapourBold.png" width="50"> Vapor Tokens <img src="./docs/VapourBold.png" width="50">


Vapor Tokens are a token-2022 compatible extension for unlinklable private transactions with plausible deniability. Build with Noir and inspired by [zERC20](https://medium.com/@intmax/zerc20-privacy-user-experience-7b7431f5b7b0) and [zkWormholes](https://eips.ethereum.org/EIPS/eip-7503).

Unlike confidential transfers which hide only the amount being transferred, vapor tokens break the link between sender and receiver like Tornado Cash or Private Cash. Unlike those protocols, Vapor Tokens make it impossible for the depositor or an observer to tell the difference between a regular transfer and a private transfer (plausible deniability).

As a token-2022 extension they maintain compatibility with existing wallet and exchange infrastructure giving you "privacy where you are".

Use cases:

- Withdraw directly from an exchange into a vapor address and then privately *condense* these funds to your cold wallet. The exchange cannot link your withdrawal to the cold wallet address.
- As a point-of-sale, create a unique payment address for each customer transaction. They can never learn anything about the finances of the shop.
- Integrate with existing DeFi protocols and pools that support Token-2022

An example token called Vapor is currently deployed to devnet with mint [4eyCrBi9Wp1TC4WwcvzYN8Ub8ZB15A5px9t7WCrgf4vn](https://solscan.io/token/4eyCrBi9Wp1TC4WwcvzYN8Ub8ZB15A5px9t7WCrgf4vn?cluster=devnet). If you would like some to test with please send your Solana devnet address on telegram to @wollumpls

See a video presentation and demo [here](https://www.youtube.com/watch?v=xPpmU1X3N6A)

> [!TIP]
> For instructions for building and running the code see [USAGE.md](./docs/USAGE.md)

## How it works

![](./docs/flow.excalidraw.svg)

### Vapor Addresses

If you want to receive funds privately you first need to generate a special kind of Solana address. This 'Vapor Address' has the following properties:

- It is indistinguishable from a normal Solana public key (i.e. it is a point on the ed25519 curve)
- You can prove that it is unspendable (i.e. that finding its corresponding private key is as hard as solving the elliptic curve discrete log problem)
- It should commit to a recipient address that is blinded by a secret value known only to the address generator

A Vapor address that meets these requirements is generated using a type of hash-to-curve as:

```js
r = random()
x = poseidon(destination || r)
y = ed25519_y_from_x(x) // y = sqrt((1 - a*x^2) / (1 - d*x^2))
P = (x, y)
addr = edwardsCompressed(P)
```

where `r` is the secret value, `destination` is the true destination we want to secretly encode in this vapor address.

When computing `y` it is possible that there is no solution. If that is the case just pick another `r` and try again. On average this should only require 2 attempts to find a valid curve point.

Any tokens transferred to this address (or its derived ATAs) will be unspendable (*vaporized*) as the private key is unknown. But to any observer this appears to be a regular transfer to a valid Solana public key (not a PDA). 

### Transfer Tree

Every time a Vapor Token is transferred a [*transfer hook*](https://solana.com/developers/guides/token-extensions/transfer-hook) records the amount and destination to a merkle tree onchain. Having an on-chain accumulator of transactions allows the ZK proofs to cheaply prove historical transfers.

### ZK Proof-of-burn

To prove a burn you must prove that:

- The address being transferred to is a vapor address and is therefore unspendable
- The transfer to that address previously occurred on the chain

To ensure we don't link the sender and receiver we want to prove the above without revealing the transfer or the vapor address itself.

In the `condenser` Noir circuit we prove exactly that. Proving the address is unspendable involves checking the point is on the curve and that its x coordinate was generated using a poseidon hash of (recipient, secret). Proving the transfer occurred is just a merkle proof to a leaf in the transfer tree. These two statements are linked by the fact that the destination of the transfer must match the vapor address.

### Double Spend Prevention

Instead of using nullifiers to prevent double spends we use another approach borrowed from zERC20. This stores on-chain the total amount withdrawn per recipient address. This is more useful than a binary nullifier as it allows using recursive proofs to wrap up a number of deposits and mint them in a single condense transaction, without revealing how many deposits are being consumed. Each new withdrawal is for the sum of all total deposits (already minted and otherwise) but the contract will only mint the difference.

Unfortunately due to limitations of Sunspot we were not able to develop the recursive proofs for this application, although they are certainly possible. This means that each vapor address should only be used once.

## How Its Built

### Solana Programs

A vapor token extends token-2022 with two programs. One is a [*transfer hook*](./anchor/programs/vaportoken-transfer-hook/) which is called on every transaction. This is what ensures that no token transfer can be made without a record of it being added to the tree. Transfer hooks are supported by most modern Solana wallets.

This is where vapor tokens differ from zERC20 by taking advantage of some unique properties of Solana. zERC20 uses a hash accumulator on-chain and proves equivalence to a merkle tree using an IVC. On Solana thanks to the Poseidon syscall and cheap execution it is possible to insert into the Merkle tree directly on-chain. This allows the protocol to be implemented with just a single proof and no IVC.

The other component is the [*condenser program*](./anchor/programs/vaportoken-condenser/). This is the mint authority for the token and is responsible for verifying the ZK proofs-of-burn, minting the corresponding new tokens, and recording the mint amounts to prevent double spend. This verifies the ZK proof using a Gnark verifier produced by the Sunspot toolchain.

### Circuit

The ZK-proof-of-burn circuit, also called the *condenser*, is written in Noir. The trickiest part of the circuit is verifying that the given vapor address was generated correctly. 

Noir conveniently has support for the ed25519 base field in the bignum library. Using this it was fairly straightforward to check that the point is on the standard ed25519 curve. Checking it is in the subgroup was a little trickier and required implementing point doubling which was done in extended form for efficiency. These two checks plus the curve paramters are implemented in [ed25519.nr](./circuits/condenser/src/ed25519.nr). This will be refactored to its own library after the hackathon as it should be quite useful for other projects working with Solana keys at a low level.

The other piece is verifying the poseidon merkle proof. This was reimplemented based on an implementation from zk-kit.noir. The existing implementation was not designed for fixed depth trees and we were able to get some efficiency improvements by fixing the depth. This is implemented in [merkle.nr](./circuits/condenser/src/merkle.nr).

### Crates

```
crates
├── cli-wallet - Generate vapor addresses, list balances, and condense from the command line
├── condenser-witness - Build the inputs for the Noir proof
├── transfer-tree - Locally reconstruct a transfer tree and build inclusion proofs
├── vaporize-addresses - Utilities for generating valid Vapor addresses
└── xtask - cargo xtask for building the verifier program using Sunspot
```

### CLI Wallet

Regular transfers and private deposits of vapor tokens can be handled by regular wallets (without the wallet or the depositor even knowing it). A special wallet is only required to condense vaporized tokens to their final destination. This is handled by the CLI wallet. This can:

- Generate new vapor addresses (for which it stores the address and secret)
- List the vapor addresses in the wallet along with any funds received
- Condense deposits by generating the zk proof-of-burn and submitting it to the condenser

Currently there is no support for Gnark proving in the browser so this needs to be a local process for now. Internally it [uses Docker to generate the Gnark proofs](./docker/prover.Dockerfile) and witness using `nargo` and `sunspot`.

## Limitations

### Balance Linkability

Similar to traditional mixers this protocol supports unlinkability but not balance confidentiality. In its current form the recommended approach is to use fixed token amounts (e.g. 0.1, 10, 100) in your transfers, and split transfers into multiple of these denominations. The anonymity set is all addresses that have received transfers of the same denomination much like a tornado cash pool. Interestingly unlike a regular mixer, fresh accounts that are just holding tokens also contribute to the anonymity set.

A significant improvement can be made by using recursive proofs to withdraw. This allows withdrawing multiple deposits in one condense action without revealing how many. Given a proof system with cheap recursion this is simple to implement and is already supported in zERC20. 

Looking further forward is no reason that this be combined with the [Solana confidential transfer standard](https://solana.com/docs/tokens/extensions/confidential-transfer) to have both. This was not implemented for the hackathon due to the lack of support by wallets and exchanges for confidential transfers. If this sees adoption then vapor tokens then support can be added for the deposits and withdrawals to be from accounts confidential balances greatly improving the unlinkability by balance.

### Single address use

Currently each generated Vapor address can only effectively be used once. They can technically be used more than once however you are only able to withdraw value equal to the largest deposit. This can also be fixed using recursive proofs to support multi-deposit withdrawals.

---

## Future Development

### Mainnet MVP

This project presents a viable protocol for building plausibly deniable unlinkability directly into tokens on Solana. For a mainnet launch the following remains outstanding:

- [ ] Audits for Solana program and Noir code
- [ ] Trusted setup for Gnark circuit
- [ ] Resursive condenser proofs to allow batch withdrawals (this also improves unlinkability by combining amounts)
- [ ] Improved wallet UX for condense workflow

> Note the trusted setup would only need to be done once for all tokens, not per token.

### Future Features

Future feature goals are:

#### Integration with Confidential Transfers

Combining vapor tokens with confidential transfers to give both unlinkability and amount confidentiality if confidentially is added to most common wallets. This would make this the most complete privacy token standard for Solana. 

#### Cross-chain Support

Given a trusted cross-chain channel to sync transfers tree roots there is no reason that the burn and the condense actions have to occur on the same chain! This would be a powerful way to build private cross-chain stablecoin transfers.

#### Stablecoin Deployment

Collaborating with a Solana stablecoin provider it would be possible to issue a USD stablecoin directly as a Vapor Token which would be ideal for daily private spending

## Thanks

Thanks to the following for making this possible:

- The original research on [ZK Wormholes](https://eips.ethereum.org/EIPS/eip-7503) by keyvank and others
- The developers of Noir-lang and the libraries used in this project
- Reilabs for their work developing [Sunspot](https://github.com/reilabs/sunspot/tree/main)
- INTMAX for their work on [zERC20](https://zerc20.io/)
- [Light Protocol](https://lightprotocol.com/) for their work on Poseidon and Merkle tree implementations for Solana that are used in this project
- [Privacy Cash](https://privacycash.us/) from which I based the on-chain Merkle tree implementation
