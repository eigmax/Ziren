use ff::PrimeField as FFPrimeField;
use p3_bn254_fr::{Bn254Fr, FFBn254Fr};
use p3_bls12381_fr::{Bls12381Fr, FFBls12381Fr};

use zkhash::{
    ark_ff::{BigInteger, PrimeField},
};

#[cfg(feature = "bn254")]
use zkhash::fields::bn256::FpBN256 as ark_FpBN256;
#[cfg(feature = "bn254")]
use zkhash::poseidon2::poseidon2_instance_bn256::RC3;

#[cfg(feature = "bls12381")]
use zkhash::fields::bls12::FpBLS12 as ark_FpBLS12;
#[cfg(feature = "bls12381")]
use zkhash::poseidon2::poseidon2_instance_bls12::RC3;

#[cfg(feature = "bn254")]
fn bn254_from_ark_ff(input: ark_FpBN256) -> Bn254Fr {
    let bytes = input.into_bigint().to_bytes_le();

    let mut res = <FFBn254Fr as ff::PrimeField>::Repr::default();

    for (i, digit) in res.as_mut().iter_mut().enumerate() {
        *digit = bytes[i];
    }

    let value = FFBn254Fr::from_repr(res);

    if value.is_some().into() {
        Bn254Fr { value: value.unwrap() }
    } else {
        panic!("Invalid field element")
    }
}

#[cfg(feature = "bn254")]
pub fn bn254_poseidon2_rc3() -> Vec<[Bn254Fr; 3]> {
    RC3.iter()
        .map(|vec| {
            vec.iter().cloned().map(bn254_from_ark_ff).collect::<Vec<_>>().try_into().unwrap()
        })
        .collect()
}

#[cfg(feature = "bn254")]
pub fn bn254_poseidon2_rc4() -> Vec<[Bn254Fr; 4]> {
    RC3.iter()
        .map(|vec| {
            let result: [Bn254Fr; 3] =
                vec.iter().cloned().map(bn254_from_ark_ff).collect::<Vec<_>>().try_into().unwrap();
            [result[0], result[1], result[2], result[2]]
        })
        .collect()
}


#[cfg(feature = "bls12381")]
fn bn254_from_ark_ff(input: ark_FpBLS12) -> Bls12381Fr {
    let bytes = input.into_bigint().to_bytes_le();

    let mut res = <FFBls12381Fr as ff::PrimeField>::Repr::default();

    for (i, digit) in res.as_mut().iter_mut().enumerate() {
        *digit = bytes[i];
    }

    let value = FFBls12381Fr::from_repr(res);

    if value.is_some().into() {
        Bls12381Fr { value: value.unwrap() }
    } else {
        panic!("Invalid field element")
    }
}

#[cfg(feature = "bls12381")]
pub fn bn254_poseidon2_rc3() -> Vec<[Bls12381Fr; 3]> {
    RC3.iter()
        .map(|vec| {
            vec.iter().cloned().map(bn254_from_ark_ff).collect::<Vec<_>>().try_into().unwrap()
        })
        .collect()
}

#[cfg(feature = "bls12381")]
pub fn bn254_poseidon2_rc4() -> Vec<[Bls12381Fr; 4]> {
    RC3.iter()
        .map(|vec| {
            let result: [Bls12381Fr; 3] =
                vec.iter().cloned().map(bn254_from_ark_ff).collect::<Vec<_>>().try_into().unwrap();
            [result[0], result[1], result[2], result[2]]
        })
        .collect()
}
