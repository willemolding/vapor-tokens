# Usage

## Prerequisites

## Deploying a new Vapor Token



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

