mod keccak;
mod serialization;

use crate::keccak::Keccak;
use ff_ext::ff::Field;
use gkr::{
    circuit::node::EvalClaim,
    poly::MultilinearPoly,
    prove_gkr,
    transcript::StdRngTranscript,
    util::{izip_eq, Itertools},
    verify_gkr,
};
use goldilocks::{Goldilocks, GoldilocksExt2};
use halo2_curves::bn256;
use keccak::dev::keccak_circuit;
use rand::{
    rngs::{OsRng, StdRng},
    RngCore, SeedableRng,
};
use serialization::SerializationEvalClaims;
use std::{error::Error, iter, panic::AssertUnwindSafe};

fn rand_vec<F: Field>(n: usize, mut rng: impl RngCore) -> Vec<F> {
    iter::repeat_with(|| F::random(&mut rng)).take(n).collect()
}

fn seeded_std_rng() -> StdRng {
    StdRng::seed_from_u64(OsRng.next_u64())
}

type GenerateProofResult = (String, Vec<u8>);

pub fn prove(input: Vec<u8>) -> Result<GenerateProofResult, Box<dyn Error>> {
    let num_reps = 1;
    let num_bits = 256;
    let keccak = Keccak::new(num_bits, num_reps);
    let (circuit, values) = keccak_circuit::<bn256::Fr, bn256::Fr>(keccak, &input);
    let mut rng = seeded_std_rng();

    let output_claims = {
        let output = &values[74];
        let point = rand_vec(output.num_vars(), &mut rng);
        let value = output.evaluate(&point);
        vec![EvalClaim::new(point, value), EvalClaim::default()]
    };

    let proof = {
        let mut transcript = StdRngTranscript::default();
        prove_gkr(&circuit, &values, &output_claims, &mut transcript).unwrap();
        transcript.into_proof()
    };

    let serialized_output_claims =
        serde_json::to_string(&SerializationEvalClaims(output_claims)).unwrap();
    Ok((serialized_output_claims, proof))
}

pub fn verify(input: Vec<u8>, output_claims: &str, proof: Vec<u8>) -> Result<bool, Box<dyn Error>> {
    let num_reps = 1;
    let num_bits = 256;
    let keccak = Keccak::new(num_bits, num_reps);
    let (circuit, values) = keccak_circuit::<bn256::Fr, bn256::Fr>(keccak, &input);
    let deserialized_eval_claims: SerializationEvalClaims =
        serde_json::from_str::<SerializationEvalClaims>(&output_claims).unwrap();
    let input_claims = {
        let mut transcript = StdRngTranscript::from_proof(&proof);
        verify_gkr::<bn256::Fr, bn256::Fr>(&circuit, &deserialized_eval_claims.0, &mut transcript)
            .unwrap()
    };

    // circuit.inputs()
    let circuit_inputs = vec![
        0, 1, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66,
        69, 72, 99, 100,
    ];

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        izip_eq!(circuit_inputs, input_claims).for_each(|(input, claims)| {
            claims
                .iter()
                .for_each(|claim| assert_eq!(values[input].evaluate(claim.point()), claim.value()))
        });
    }));

    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
pub mod test {
    use crate::{prove, verify};

    #[test]
    fn test_prove_and_verify_keccak() {
        let input: Vec<u8> = vec![
            116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let (output_claims, proof) = prove(input.clone()).unwrap();
        let result = verify(input, &output_claims, proof).unwrap();
        assert_eq!(result, true);
    }
}
