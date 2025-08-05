use p3_bn254_fr::Bn254Fr;
use p3_bls12381_fr::Bls12381Fr;
use p3_field::extension::BinomialExtensionField;
use p3_koala_bear::KoalaBear;
use zkm_stark::{InnerChallenge, InnerVal};

use crate::{circuit::AsmConfig, prelude::Config};

pub type InnerConfig = AsmConfig<InnerVal, InnerChallenge>;

#[derive(Clone, Default, Debug)]
pub struct OuterConfig;

#[cfg(feature = "bn254")]
impl Config for OuterConfig {
    type N = Bn254Fr;
    type F = KoalaBear;
    type EF = BinomialExtensionField<KoalaBear, 4>;
}

#[cfg(feature = "bls12381")]
impl Config for OuterConfig {
    type N = Bls12381Fr;
    type F = KoalaBear;
    type EF = BinomialExtensionField<KoalaBear, 4>;
}
