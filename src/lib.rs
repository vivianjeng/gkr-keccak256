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
use keccak::dev::keccak_circuit;
use rand::{
    rngs::{OsRng, StdRng},
    RngCore, SeedableRng,
};
use std::iter;

mod keccak;

fn rand_vec<F: Field>(n: usize, mut rng: impl RngCore) -> Vec<F> {
    iter::repeat_with(|| F::random(&mut rng)).take(n).collect()
}

fn seeded_std_rng() -> StdRng {
    StdRng::seed_from_u64(OsRng.next_u64())
}

pub fn prove_keccak(input: &[u8]) -> (Vec<EvalClaim<GoldilocksExt2>>, Vec<u8>) {
    let num_reps = 1;
    let num_bits = 256;
    let keccak = Keccak::new(num_bits, num_reps);
    let (circuit, values) = keccak_circuit::<Goldilocks, GoldilocksExt2>(keccak, &input);
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
    (output_claims, proof)
}

pub fn verify_keccak(input: &[u8], output_claims: Vec<EvalClaim<GoldilocksExt2>>, proof: &[u8]) {
    let num_reps = 1;
    let num_bits = 256;
    let keccak = Keccak::new(num_bits, num_reps);
    let (circuit, values) = keccak_circuit::<Goldilocks, GoldilocksExt2>(keccak, &input);
    let input_claims = {
        let mut transcript = StdRngTranscript::from_proof(&proof);
        verify_gkr(&circuit, &output_claims, &mut transcript).unwrap()
    };

    // circuit.inputs()
    let circuit_inputs = vec![
        0, 1, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66,
        69, 72, 99, 100,
    ];

    izip_eq!(circuit_inputs, input_claims).for_each(|(input, claims)| {
        claims
            .iter()
            .for_each(|claim| assert_eq!(values[input].evaluate(claim.point()), claim.value()))
    });
}

#[cfg(test)]
pub mod test {
    use crate::{prove_keccak, verify_keccak};

    #[test]
    fn test_prove_and_verify_keccak() {
        let input: Vec<u8> = vec![
            116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let (output_claims, proof) = prove_keccak(&input);
        verify_keccak(&input, output_claims, &proof);
    }
}
