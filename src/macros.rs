// -*- coding: utf-8; mode: rust; -*-
//
// To the extent possible under law, the authors have waived all
// copyright and related or neighboring rights to zkp,
// using the Creative Commons "CC0" public domain dedication.  See
// <http://creativecommons.org/publicdomain/zero/1.0/> for full
// details.
//
// Authors:
// - Henry de Valence <hdevalence@hdevalence.ca>

#[doc(hidden)]
#[macro_export]
macro_rules! __compute_formula_scalarlist {
    // Unbracket a statement
    (($publics:ident, $scalars:ident) ($($x:tt)*)) => {
        // Add a trailing +
        __compute_formula_scalarlist!(($publics,$scalars) $($x)* +)
    };
    // Inner part of the formula: give a list of &Scalars
    // Since there's a trailing +, we can just generate the list as normal...
    (($publics:ident, $scalars:ident)
     $( $point:ident * $scalar:ident +)+ ) => {
        &[ $( $scalars.$scalar ,)* ]
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __compute_formula_pointlist {
    // Unbracket a statement
    (($publics:ident, $scalars:ident) ($($x:tt)*)) => {
        // Add a trailing +
        __compute_formula_pointlist!(($publics,$scalars) $($x)* +)
    };
    // Inner part of the formula: give a list of &Scalars
    // Since there's a trailing +, we can just generate the list as normal...
    (($publics:ident, $scalars:ident)
     $( $point:ident * $scalar:ident +)* ) => {
        &[ $( *($publics.$point) ,)* ]
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __compute_commitments_consttime {
    (($publics:ident, $scalars:ident) $($lhs:ident = $statement:tt),+) => {
        Commitments {
            $( $lhs :
               RistrettoPoint::multiscalar_mul(
                   __compute_formula_scalarlist!(($publics, $scalars) $statement),
                   __compute_formula_pointlist!(($publics, $scalars) $statement),
               )
            ),+
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __recompute_commitments_vartime {
    (($publics:ident, $scalars:ident, $minus_c:ident) $($lhs:ident = $statement:tt),+) => {
        Commitments {
            $( $lhs :
               RistrettoPoint::vartime_multiscalar_mul(
                   __compute_formula_scalarlist!(($publics, $scalars) $statement)
                       .into_iter()
                       .chain(iter::once(&($minus_c)))
                   ,
                   __compute_formula_pointlist!(($publics, $scalars) $statement)
                       .into_iter()
                       .chain(iter::once($publics.$lhs))
               )
            ),+
        }
    }
}

/// Creates a module with code required to produce a non-interactive
/// zero-knowledge proof statement, to serialize it to wire format, to
/// parse from wire format, and to verify the proof statement.
///
/// The statement is specified in an embedded DSL resembling
/// Camenisch-Stadler notation.  For instance, a proof of knowledge of
/// two equal discrete logarithms ("DLEQ") is specified as:
///
/// ```rust,ignore
/// create_nipk!{dleq, (x), (A, B, G, H) : A = (G * x), B = (H * x) }
/// ```
///
/// This creates a module `dleq` with code for proving knowledge of a
/// secret `x: Scalar` such that `A = G*x`, `B = H*x` for public
/// parameters `A, B, G, H: RistrettoPoint`.  In general the syntax is
///
/// ```rust,ignore
/// create_nipk!{
///     module_name, // used to label proof statements
///     (x,y,z,...), // secret variable names
///     (A,B,C,...)  // public parameter names
///     :
///     LHS = (A * x + B * y + C * z + ... ),  // comma-seperated statements
///     ...
/// }
/// ```
///
/// Statements have the form `LHS = (A * x + B * y + C * z + ... )`,
/// where `LHS` is one of the points listed as a public parameter, and
/// the right-hand side is a sum of public points multiplied by secret
/// scalars.
///
/// Inside the generated module `module_name`, the macro defines three
/// structs:
///
/// A `Publics` struct corresponding to the public parameters, of the
/// form
///
/// ```rust,ignore
/// pub struct Publics<'a> { pub A: &'a RistrettoPoint, ... }
/// ```
///
/// A `Secrets` struct corresponding to the secret parameters, of the
/// form
///
/// ```rust,ignore
/// pub struct Secrets<'a> { pub x: &'a Scalar, ... }
/// ```
///
/// A `Proof` struct, of the form
///
/// ```rust,ignore
/// #[derive(Serialize, Deserialize)]
/// pub struct Proof { ... }
///
/// impl Proof {
///     pub fn create(
///         transcript: &mut Transcript,
///         publics: Publics,
///         secrets: Secrets,
///     ) -> Proof { ... }
///
///     pub fn verify(
///         &self,
///         &mut Transcript,
///         publics: Publics,
///     ) -> Result<(),()> { ... }
/// }
/// ```
///
/// The `Proof` struct derives the Serde traits, so it can be
/// serialized and deserialized to various wire formats.
///
/// The `Publics` and `Secrets` structs are used to fake named
/// arguments in the input to `create` and `verify`.  Proof creation
/// is done in constant time.  Proof verification uses variable-time
/// code.
///
/// As an example, we can create and verify a DLEQ proof as follows:
///
/// ```
/// #[macro_use]
/// extern crate serde_derive;
///
/// #[macro_use]
/// extern crate zkp;
/// use zkp::Transcript;
///
/// extern crate curve25519_dalek;
/// use curve25519_dalek::constants as dalek_constants;
/// use curve25519_dalek::ristretto::RistrettoPoint;
/// use curve25519_dalek::scalar::Scalar;
///
/// extern crate sha2;
/// use sha2::Sha512;
///
/// extern crate bincode;
///
/// // ...
///
/// # fn main() {
/// let G = &dalek_constants::RISTRETTO_BASEPOINT_POINT;
/// let H = RistrettoPoint::hash_from_bytes::<Sha512>(G.compress().as_bytes());
///
/// create_nipk!{dleq, (x), (A, B, G, H) : A = (G * x), B = (H * x) }
///
/// let x = Scalar::from(89327492234u64);
/// let A =  G * &x;
/// let B = &H * &x;
///
/// let publics = dleq::Publics{A: &A, B: &B, G: G, H: &H};
/// let secrets = dleq::Secrets{x: &x};
///
/// let mut transcript = Transcript::new(b"DLEQExample");
/// let proof = dleq::Proof::create(&mut transcript, publics, secrets);
///
/// // Serialize to bincode representation
/// let proof_bytes = bincode::serialize(&proof).unwrap();
///
/// // Send bytes over the wire here ...
///
/// // Parse bytes back to in-memory representation
/// let parsed_proof: dleq::Proof
///     = bincode::deserialize(&proof_bytes).unwrap();
///
/// // Check the proof with a fresh transcript.
/// let mut transcript = Transcript::new(b"DLEQExample");
/// assert!(parsed_proof.verify(&mut transcript, publics).is_ok());
/// # }
/// ```
#[macro_export]
macro_rules! create_nipk {
    (
        $proof_module_name:ident // Name of the module to create
        ,
        ( $($secret:ident),+ ) // Secret variables, sep by commas
        ,
        ( $($public:ident),+ ) // Public variables, sep by commas
        :
        // List of statements to prove
        // Format: LHS = ( ... RHS expr ... ),
        $($lhs:ident = $statement:tt),+
    ) => {
        pub mod $proof_module_name {
            use $crate::curve25519_dalek::scalar::Scalar;
            use $crate::curve25519_dalek::ristretto::RistrettoPoint;
            use $crate::curve25519_dalek::traits::{MultiscalarMul, VartimeMultiscalarMul};
            use $crate::merlin::Transcript;
            use $crate::rand::thread_rng;

            use std::iter;

            #[derive(Copy, Clone)]
            pub struct Secrets<'a> {
                // Create a parameter for each secret value
                $(
                    pub $secret : &'a Scalar,
                )+
            }

            #[derive(Copy, Clone)]
            pub struct Publics<'a> {
                // Create a parameter for each public value
                $(
                    pub $public : &'a RistrettoPoint,
                )+
            }

            // Hack because we can't concat identifiers,
            // so do responses.x instead of responses_x
            // rand.x instead of rand_x, etc.

            struct Commitments {$($lhs: RistrettoPoint,)+ }
            struct Randomnesses {$($secret : Scalar,)+}
            #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
            struct Responses {$($secret : Scalar,)+}

            #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
            pub struct Proof {
                challenge: Scalar,
                responses: Responses,
            }

            impl Proof {
                /// Create a `Proof` from the given `Publics` and `Secrets`.
                #[allow(dead_code)]
                pub fn create(
                    transcript: &mut Transcript,
                    publics: Publics,
                    secrets: Secrets,
                ) -> Proof {
                    // Commit public data to the transcript.
                    transcript.commit_bytes(b"domain-sep", stringify!($proof_module_name).as_bytes());
                    $(
                        transcript.commit_bytes(
                            stringify!($public).as_bytes(),
                            publics.$public.compress().as_bytes(),
                        );
                    )+

                    // Construct a TranscriptRng
                    let rng_builder = transcript.build_rng();
                    $(
                        let rng_builder = rng_builder.commit_witness_bytes(
                            stringify!($secret).as_bytes(),
                            secrets.$secret.as_bytes(),
                        );
                    )+
                    let mut transcript_rng = rng_builder.finalize(&mut thread_rng());

                    // Use the TranscriptRng to generate blinding factors
                    let rand = Randomnesses{
                        $(
                            $secret : Scalar::random(&mut transcript_rng),
                        )+
                    };

                    // Form a commitment to the blinding factors for each statement LHS
                    //
                    // $statement_rhs = `X * x + Y * y + Z * z`
                    // should become
                    // `publics.X * rand.x + publics.Y * rand.y + publics.Z * rand.z`
                    let commitments = __compute_commitments_consttime!(
                        (publics, rand) $($lhs = $statement),*
                    );

                    // Add all commitments to the transcript
                    $(
                        transcript.commit_bytes(
                            stringify!(com $lhs).as_bytes(),
                            commitments.$lhs.compress().as_bytes(),
                        );
                    )+

                    // Obtain a scalar challenge
                    let challenge = {
                        let mut bytes = [0; 64];
                        transcript.challenge_bytes(b"chal", &mut bytes);
                        Scalar::from_bytes_mod_order_wide(&bytes)
                    };

                    let responses = Responses{
                        $(
                            $secret : &(&challenge * secrets.$secret) + &rand.$secret,
                        )+
                    };

                    Proof{ challenge: challenge, responses: responses }
                }

                /// Verify the `Proof` using the public parameters `Publics`.
                #[allow(dead_code)]
                pub fn verify(
                    &self,
                    transcript: &mut Transcript,
                    publics: Publics,
                ) -> Result<(),()> {
                    // Recompute the prover's commitments based on their claimed challenge value:
                    // `A = X * x + Y * y`
                    // should become
                    // `publics.X * responses.x + publics.Y * responses.y - publics.A * self.challenge`
                    let responses = &self.responses;
                    let minus_c = -&self.challenge;
                    let commitments = __recompute_commitments_vartime!(
                        (publics, responses, minus_c) $($lhs = $statement),*
                    );

                    // Commit public data to the transcript.
                    transcript.commit_bytes(b"domain-sep", stringify!($proof_module_name).as_bytes());
                    $(
                        transcript.commit_bytes(
                            stringify!($public).as_bytes(),
                            publics.$public.compress().as_bytes(),
                        );
                    )+

                    // Commit the recomputed commitments to the transcript.
                    $(
                        transcript.commit_bytes(
                            stringify!(com $lhs).as_bytes(),
                            commitments.$lhs.compress().as_bytes(),
                        );
                    )+

                    // Recompute challenge
                    let challenge = {
                        let mut bytes = [0; 64];
                        transcript.challenge_bytes(b"chal", &mut bytes);
                        Scalar::from_bytes_mod_order_wide(&bytes)
                    };

                    if challenge == self.challenge { Ok(()) } else { Err(()) }
                }
            }

            #[cfg(all(feature = "bench", test))]
            mod bench {
                extern crate test;

                use $crate::rand::OsRng;

                use super::*;

                use self::test::Bencher;

                #[bench]
                #[allow(dead_code)]
                fn create(b: &mut Bencher) {
                    let mut csprng = OsRng::new().unwrap();

                    // Need somewhere to actually put the public points
                    struct DummyPublics { $( pub $public : RistrettoPoint, )+ }
                    let dummy_publics = DummyPublics {
                        $( $public : RistrettoPoint::random(&mut csprng) , )+
                    };

                    let publics = Publics {
                        $( $public : &dummy_publics.$public , )+
                    };

                    struct DummySecrets { $( pub $secret : Scalar, )+ }
                    let dummy_secrets = DummySecrets {
                        $( $secret : Scalar::random(&mut csprng) , )+
                    };

                    let secrets = Secrets {
                        $( $secret : &dummy_secrets.$secret , )+
                    };

                    b.iter(|| {
                        let mut transcript = Transcript::new(b"BenchmarkProve");
                        Proof::create(&mut transcript, publics, secrets)
                    });
                }

                #[bench]
                #[allow(dead_code)]
                fn verify(b: &mut Bencher) {
                    let mut csprng = OsRng::new().unwrap();

                    // Need somewhere to actually put the public points
                    struct DummyPublics { $( pub $public : RistrettoPoint, )+ }
                    let dummy_publics = DummyPublics {
                        $( $public : RistrettoPoint::random(&mut csprng) , )+
                    };

                    let publics = Publics {
                        $( $public : &dummy_publics.$public , )+
                    };

                    struct DummySecrets { $( pub $secret : Scalar, )+ }
                    let dummy_secrets = DummySecrets {
                        $( $secret : Scalar::random(&mut csprng) , )+
                    };

                    let secrets = Secrets {
                        $( $secret : &dummy_secrets.$secret , )+
                    };

                    let mut transcript = Transcript::new(b"BenchmarkVerify");
                    let p = Proof::create(&mut transcript, publics, secrets);

                    b.iter(|| {
                        let mut transcript = Transcript::new(b"BenchmarkVerify");
                        p.verify(&mut transcript, publics).is_ok()
                    });
                }
            }
        }
    }
}