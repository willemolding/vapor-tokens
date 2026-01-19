#!/bin/sh
set -eu

cd ./circuits/condenser

# Write the inputs
cat > Prover.toml

# create the witness
nargo execute 1>&2

# generate the gnark proof
sunspot prove ./target/condenser.json ./target/condenser.gz ./target/condenser.ccs ./target/condenser.pk 1>&2

# Output the proof and witness
printf -- '%s\n' '---PROOF---'
cat ./target/condenser.proof
printf -- '\n%s\n' '---WITNESS---'
cat ./target/condenser.pw
