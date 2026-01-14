
execute:
    cd circuits/condenser && nargo execute

build_acir:
    cd circuits/condenser && nargo build

build_gnark: build_acir
    cd circuits/condenser && sunspot compile ./target/condenser.json

trusted_setup: build_gnark
    cd circuits/condenser && sunspot setup ./target/condenser.ccs

gnark_prove: trusted_setup
    cd circuits/condenser && sunspot prove ./target/condenser.json ./target/condenser.gz ./target/condenser.ccs ./target/condenser.pk

build_verifier_program: trusted_setup
    cargo xtask codegen
