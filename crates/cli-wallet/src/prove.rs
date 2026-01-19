use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use condenser_witness::CondenserWitness;

const DEFAULT_PROVER_IMAGE: &str = "vapor-prover:latest";
const PROVER_IMAGE_ENV: &str = "VAPOR_PROVER_IMAGE";
const PROOF_MARKER: &[u8] = b"---PROOF---\n";
const WITNESS_MARKER: &[u8] = b"\n---WITNESS---\n";

pub fn prove<const HEIGHT: usize>(
    input: CondenserWitness<HEIGHT>,
) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let toml = input.to_toml();
    let root_dir = workspace_root()?;
    let circuits_dir = root_dir.join("circuits").join("condenser");

    let stdout = run_prover_container(&circuits_dir, &toml)?;
    parse_prover_stdout(&stdout)
}

fn run_prover_container(circuits_dir: &Path, toml: &str) -> anyhow::Result<Vec<u8>> {
    let image =
        std::env::var(PROVER_IMAGE_ENV).unwrap_or_else(|_| DEFAULT_PROVER_IMAGE.to_string());
    let volume = format!("{}:/circuits/condenser", circuits_dir.display());

    let mut child = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-i")
        .arg("-v")
        .arg(volume)
        .arg(image)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(toml.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("prover container failed with status {}", output.status);
    }

    Ok(output.stdout)
}

fn workspace_root() -> anyhow::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| anyhow::anyhow!("failed to resolve workspace root"))?;
    Ok(root.to_path_buf())
}

fn parse_prover_stdout(stdout: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let proof_start = find_subslice(stdout, PROOF_MARKER)
        .ok_or_else(|| anyhow::anyhow!("missing proof marker in prover output"))?
        + PROOF_MARKER.len();
    let witness_marker_pos = find_subslice(&stdout[proof_start..], WITNESS_MARKER)
        .map(|pos| proof_start + pos)
        .ok_or_else(|| anyhow::anyhow!("missing witness marker in prover output"))?;

    let proof_bytes = stdout[proof_start..witness_marker_pos].to_vec();
    let witness_start = witness_marker_pos + WITNESS_MARKER.len();
    let witness_bytes = stdout[witness_start..].to_vec();

    if proof_bytes.is_empty() || witness_bytes.is_empty() {
        anyhow::bail!("prover output did not include proof or witness bytes");
    }

    Ok((proof_bytes, witness_bytes))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
