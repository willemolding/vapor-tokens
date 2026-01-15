# Vapor Tokens

Vapor Tokens are a token-2022 compatible extension for plausibly deniable, unlinklable private transactions. Inspired by [zERC20](https://medium.com/@intmax/zerc20-privacy-user-experience-7b7431f5b7b0) and [zkWormholes](https://eips.ethereum.org/EIPS/eip-7503).

Unlike confidential transfers which hide only the amount being transferred, vapor tokens break the link between sender and receiver like Tornado Cash or Private Cash. Unlike those protocols, Vapor Tokens make it impossible for an observer to tell the difference between a regular transfer and a private transfer.

As a token-2022 extension they maintain compatibility with existing wallet and exchange infrastructure giving you "privacy where you are".

Use cases:

- Withdraw directly from an exchange into a vapor address and then privately *condense* these funds to your cold wallet. The exchange cannot link your withdrawal to the cold wallet address.
- Receive donations anonymously by publishing a vapor address online
- Integrate with existing DeFi protocols and pools that support Token-2022

## How it works

### Vapor Addresses

If you want to receive funds privately you first need to generate a special kind of Solana address. This 'Vapor Address' has the following properties:

- It is indistinguishable from a normal Solana public key (i.e. it is a point on the ed25519 curve)
- You can prove that it is unspendable (i.e. finding its corresponding private key is as hard as solving the elliptic curve discrete log problem)
- It should commit to a recipient address that is blinded by a secret value known only to the address generator

A Vapor address that meets these requirements is generated using kind of hash-to-curve as:

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

Every time a Vapor Token is transferred a [transfer hook](https://solana.com/developers/guides/token-extensions/transfer-hook) is called which adds the amount and destination to a merkle tree onchain. Having an on-chain accumulator of transactions allows the ZK proofs to cheaply prove something about a historical transfer.

### ZK Proof-of-burn

To prove a burn you just have to prove that:

- The address being transferred to was generated using the above process and is therefore unspendable
- A transfer previously occurred on the chain to that address

To ensure we don't link the sender and receiver we want to prove the above without revealing the exact transfer.

In the `condenser` Noir circuit we prove exactly that. Proving the address is unspendable involves checking the point is on the curve and that its x coordinate was generated using a poseidon hash of (recipient, secret). Proving the transfer occurred is just a merkle proof to a leaf in the transfer tree. These two statements are linked by the fact that the destination of the transfer must match the vapor address.

### Double Spend Prevention

Instead of using nullifiers to prevent double spends we use another approach borrowed from zERC20.  

---

## Future Development

This project presents a very viable protocol for building unlinkability directly into tokens on Solana. The following remains outstanding:

- [ ] Solana program and Noir code audits
- [ ] Trusted setup for Gnark circuit
- [ ] Resursive condenser proofs to allow batch withdrawals (this also improves unlinkability by combining amounts)
- [ ] Improved wallet UX for condense workflow

> Note the trusted setup would only need to be done once for all tokens, not per token.

Vapor Tokens could be combined with [confidential transfers](https://solana.com/docs/tokens/extensions/confidential-transfer) to give both unlinkability and amount confidentiality. This wasn't pursued for the hackathon due to limited wallet support at this time but in the future would be the ultimate private tokens standard for Solana!
