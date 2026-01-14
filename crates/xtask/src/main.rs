use gnark_verifier_solana::vk::generate_key_file;

fn main() {
    let command = std::env::args().nth(1).expect("expected first arg");

    match command.as_str() {
        "codegen" => codegen(),
        _ => {
            panic!("unknown command: {}", command);
        }
    }
}

fn codegen() {
    let vk_path = "circuits/condenser/target/condenser.vk";
    let out_path = "anchor/programs/vaportoken-condenser/src/vk.rs";
    generate_key_file(vk_path, out_path).expect("failed to generate vk/pk files");
}
