//! Utility to format Groth16 proof for Solidity contract submission
//!
//! This tool extracts A, B, C points from a Groth16 proof and formats them
//! for submission to the VerifierContract.submitBlockProof function.

use ark_bn254::{g1::G1Affine, g2::G2Affine, Bn254};
use ark_groth16::Proof;
use ark_serialize::{CanonicalDeserialize, Compress, Validate};
use std::fs;
use std::io::Write;

#[derive(serde::Serialize, serde::Deserialize)]
struct SnarkProofWrapper {
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    version: u8,
}

fn format_g1_point(point: &G1Affine) -> String {
    let x = point.x;
    let y = point.y;
    format!(
        "({}, {})",
        hex::encode(x.to_bytes_le()),
        hex::encode(y.to_bytes_le())
    )
}

fn format_g2_point(point: &G2Affine) -> String {
    let x = point.x;
    let y = point.y;
    format!(
        "(({}, {}), ({}, {}))",
        hex::encode(x.c0.to_bytes_le()),
        hex::encode(x.c1.to_bytes_le()),
        hex::encode(y.c0.to_bytes_le()),
        hex::encode(y.c1.to_bytes_le())
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <proof_file> [output_file]", args[0]);
        eprintln!("  proof_file: Path to serialized Groth16 proof (bincode format)");
        eprintln!("  output_file: Optional path to output file (default: stdout)");
        std::process::exit(1);
    }

    let proof_file = &args[1];
    let output_file = args.get(2);

    // Read proof from file
    let proof_data = fs::read(proof_file)?;
    let wrapper: SnarkProofWrapper = bincode::deserialize(&proof_data)
        .map_err(|e| format!("Failed to deserialize proof wrapper: {}", e))?;

    if wrapper.version != 3 {
        return Err(format!("Unsupported proof version: {}", wrapper.version).into());
    }

    // Deserialize Groth16 proof
    let groth16_proof =
        Proof::<Bn254>::deserialize_with_mode(&wrapper.proof[..], Compress::Yes, Validate::Yes)
            .map_err(|e| format!("Failed to deserialize Groth16 proof: {}", e))?;

    // Extract A, B, C points
    let a = groth16_proof.a;
    let b = groth16_proof.b;
    let c = groth16_proof.c;

    // Format for Solidity
    // A (G1): 64 bytes (32 X + 32 Y)
    // B (G2): 128 bytes (64 X + 64 Y)
    // C (G1): 64 bytes (32 X + 32 Y)
    // Total: 256 bytes

    let mut solidity_proof = Vec::new();

    // A point (G1)
    solidity_proof.extend_from_slice(&a.x.to_bytes_le());
    solidity_proof.extend_from_slice(&a.y.to_bytes_le());

    // B point (G2)
    solidity_proof.extend_from_slice(&b.x.c0.to_bytes_le());
    solidity_proof.extend_from_slice(&b.x.c1.to_bytes_le());
    solidity_proof.extend_from_slice(&b.y.c0.to_bytes_le());
    solidity_proof.extend_from_slice(&b.y.c1.to_bytes_le());

    // C point (G1)
    solidity_proof.extend_from_slice(&c.x.to_bytes_le());
    solidity_proof.extend_from_slice(&c.y.to_bytes_le());

    // Convert public inputs to 24 field elements
    // Each 32-byte root = 8 field elements (4 bytes each, little-endian)
    // Format: prev_state_root (32 bytes) + new_state_root (32 bytes) + withdrawals_root (32 bytes)
    // Total: 96 bytes = 24 u32 values

    if wrapper.public_inputs.len() < 96 {
        return Err(format!(
            "Invalid public inputs length: expected 96 bytes, got {}",
            wrapper.public_inputs.len()
        )
        .into());
    }

    let mut public_inputs_elements = Vec::new();

    // Process each root (32 bytes = 8 u32 values)
    for root_idx in 0..3 {
        let root_start = root_idx * 32;
        for i in 0..8 {
            let byte_start = root_start + (i * 4);
            let chunk = &wrapper.public_inputs[byte_start..byte_start + 4];
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            public_inputs_elements.push(value);
        }
    }

    // Format output
    let mut output = String::new();
    output.push_str("// Groth16 Proof for Solidity\n");
    output.push_str("// Generated from: ");
    output.push_str(proof_file);
    output.push_str("\n\n");

    output.push_str("// Proof (256 bytes): A (64) + B (128) + C (64)\n");
    output.push_str("const proof = \"0x");
    output.push_str(&hex::encode(&solidity_proof));
    output.push_str("\";\n\n");

    output.push_str("// Public Inputs (24 uint256 elements)\n");
    output.push_str("// Format: prev_state_root (8) + new_state_root (8) + withdrawals_root (8)\n");
    output.push_str("const publicInputs = [\n");
    for (i, elem) in public_inputs_elements.iter().enumerate() {
        output.push_str(&format!("  \"{}\"", elem));
        if i < public_inputs_elements.len() - 1 {
            output.push_str(",");
        }
        output.push_str("\n");
    }
    output.push_str("];\n\n");

    output.push_str("// Proof components (for debugging):\n");
    output.push_str(&format!("// A (G1): {}\n", format_g1_point(&a)));
    output.push_str(&format!("// B (G2): {}\n", format_g2_point(&b)));
    output.push_str(&format!("// C (G1): {}\n", format_g1_point(&c)));

    // Write output
    if let Some(output_path) = output_file {
        fs::write(output_path, output)?;
        println!("✅ Proof formatted and saved to: {}", output_path);
    } else {
        print!("{}", output);
    }

    Ok(())
}
