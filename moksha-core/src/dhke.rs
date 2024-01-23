//! This module defines the basic crypto primitives for Blind Diffie-Hellman Key Exchange (blind ecash).
//!
//! Implementation of [RubenSomsen/Blind-DH-ecash.md](https://gist.github.com/RubenSomsen/be7a4760dd4596d06963d67baf140406)
//!
//! Bob (Mint):
//!```python
//! A = a*G
//! return A
//! ```
//!
//!
//! Alice (Client):
//!```python
//! Y = hash_to_curve(secret_message)
//! r = random blinding factor
//! B'= Y + r*G
//! return B'
//! ```
//!
//!
//! Bob:
//!```python
//! C' = a*B'
//! (= a*Y + a*r*G)
//! return C'
//!```
//!
//! Alice:
//!```python
//! C = C' - r*A
//! (= C' - a*r*G)
//! (= a*Y)
//! return C, secret_message
//!```
//!
//! Bob:
//!```python
//! Y = hash_to_curve(secret_message)
//! C == a*Y
//!```
//! If true, C must have originated from Bob
//!
use crate::error::MokshaCoreError;
use bitcoin_hashes::{sha256, Hash};
use secp256k1::{All, PublicKey, Scalar, Secp256k1, SecretKey};
use std::iter::once;

#[derive(Clone, Debug)]
pub struct Dhke {
    secp: Secp256k1<All>,
}

impl Default for Dhke {
    fn default() -> Self {
        Self::new()
    }
}

impl Dhke {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    fn get_hash(message: &[u8]) -> Vec<u8> {
        let hash = sha256::Hash::hash(message);
        hash.as_byte_array().to_vec()
    }

    /// Generates a point from the message hash and checks if the point lies on the curve.
    /// If it does not, iteratively tries to compute a new point from the hash.
    fn hash_to_curve(message: &[u8]) -> PublicKey {
        let mut point: Option<PublicKey> = None;
        let mut msg_to_hash = message.to_vec();
        while point.is_none() {
            let hash = Self::get_hash(&msg_to_hash);
            let input = &once(&0x02).chain(hash.iter()).cloned().collect::<Vec<u8>>();
            PublicKey::from_slice(input).map_or_else(|_| msg_to_hash = hash, |p| point = Some(p))
        }
        point.unwrap()
    }

    pub fn step1_alice(
        &self,
        secret_msg: impl Into<String>,
        blinding_factor: Option<&[u8]>,
    ) -> Result<(PublicKey, SecretKey), MokshaCoreError> {
        let mut rng = rand::thread_rng();

        let y = Self::hash_to_curve(secret_msg.into().as_bytes());
        let secret_key = match blinding_factor {
            Some(f) => SecretKey::from_slice(f)?,
            None => SecretKey::new(&mut rng),
        };
        let b = y.combine(&PublicKey::from_secret_key(&self.secp, &secret_key))?;
        Ok((b, secret_key))
    }

    pub fn step2_bob(&self, b: PublicKey, a: &SecretKey) -> Result<PublicKey, MokshaCoreError> {
        b.mul_tweak(&self.secp, &Scalar::from(*a))
            .map_err(MokshaCoreError::Secp256k1Error)
    }

    pub fn step3_alice(
        &self,
        c_: PublicKey,
        r: SecretKey,
        a: PublicKey,
    ) -> Result<PublicKey, MokshaCoreError> {
        c_.combine(
            &a.mul_tweak(&self.secp, &Scalar::from(r))
                .map_err(MokshaCoreError::Secp256k1Error)?
                .negate(&self.secp),
        )
        .map_err(MokshaCoreError::Secp256k1Error)
    }

    pub fn verify(
        &self,
        a: SecretKey,
        c: PublicKey,
        secret_msg: impl Into<String>,
    ) -> Result<bool, MokshaCoreError> {
        let y = Self::hash_to_curve(secret_msg.into().as_bytes());
        Some(c == y.mul_tweak(&self.secp, &Scalar::from(a))?).ok_or(
            MokshaCoreError::Secp256k1Error(secp256k1::Error::InvalidPublicKey),
        )
    }
}

pub fn public_key_from_hex(hex: &str) -> secp256k1::PublicKey {
    use hex::FromHex;
    let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
    secp256k1::PublicKey::from_slice(&input_vec).expect("Invalid Public Key")
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::dhke::{public_key_from_hex, Dhke};
    use anyhow::Ok;
    use pretty_assertions::assert_eq;

    fn hex_to_string(hex: &str) -> String {
        use hex::FromHex;
        let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
        String::from_utf8(input_vec).expect("Invalid UTF-8 String")
    }

    fn pk_from_hex(hex: &str) -> secp256k1::SecretKey {
        secp256k1::SecretKey::from_str(hex).expect("Invalid SecretKey")
    }

    #[test]
    fn test_hash_to_curve_zero() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000000");
        let expected_result = "0266687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925";

        let pk = Dhke::hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_hash_to_curve_zero_one() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000001");
        let expected_result = "02ec4916dd28fc4c10d78e287ca5d9cc51ee1ae73cbfde08c6b37324cbfaac8bc5";

        let pk = Dhke::hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_hash_to_curve_iterate() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000002");
        let expected_result = "02076c988b353fcbb748178ecb286bc9d0b4acf474d4ba31ba62334e46c97c416a";

        let pk = Dhke::hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_step1_alice() -> anyhow::Result<()> {
        let dhke = Dhke::new();
        let blinding_factor =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000001");
        let (pub_key, secret_key) =
            dhke.step1_alice("test_message", Some(blinding_factor.as_bytes()))?;
        let pub_key_str = pub_key.to_string();

        assert_eq!(
            pub_key_str,
            "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2"
        );

        assert_eq!(
            hex::encode(secret_key.secret_bytes()),
            "0000000000000000000000000000000000000000000000000000000000000001"
        );
        Ok(())
    }

    #[test]
    fn test_step2_bob() -> anyhow::Result<()> {
        let dhke = Dhke::new();
        let blinding_factor =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000001");
        let (pub_key, _) = dhke.step1_alice("test_message", Some(blinding_factor.as_bytes()))?;

        let a = pk_from_hex("0000000000000000000000000000000000000000000000000000000000000001");

        let c = dhke.step2_bob(pub_key, &a)?;
        let c_str = c.to_string();
        assert_eq!(
            "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2".to_string(),
            c_str
        );

        Ok(())
    }

    #[test]
    fn test_step3_alice() -> anyhow::Result<()> {
        let dhke = Dhke::new();
        let c_ = public_key_from_hex(
            "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
        );

        let r = pk_from_hex("0000000000000000000000000000000000000000000000000000000000000001");

        let a = public_key_from_hex(
            "020000000000000000000000000000000000000000000000000000000000000001",
        );

        let result = dhke.step3_alice(c_, r, a)?;
        assert_eq!(
            "03c724d7e6a5443b39ac8acf11f40420adc4f99a02e7cc1b57703d9391f6d129cd".to_string(),
            result.to_string()
        );
        Ok(())
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_verify() -> anyhow::Result<()> {
        // # a = PrivateKey()
        // # A = a.pubkey
        // # secret_msg = "test"
        // # B_, r = step1_alice(secret_msg)
        // # C_ = step2_bob(B_, a)
        // # C = step3_alice(C_, r, A)
        // # print("C:{}, secret_msg:{}".format(C, secret_msg))
        // # assert verify(a, C, secret_msg)
        // # assert verify(a, C + C, secret_msg) == False  # adding C twice shouldn't pass
        // # assert verify(a, A, secret_msg) == False  # A shouldn't pass

        let dhke = Dhke::new();

        // Generate Alice's private key and public key
        let a = pk_from_hex("0000000000000000000000000000000000000000000000000000000000000001");
        let A = a.public_key(&dhke.secp);

        let blinding_factor =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000002");

        // Generate a shared secret
        let secret_msg = "test";
        let (B_, r) = dhke.step1_alice(secret_msg, Some(blinding_factor.as_bytes()))?;
        let C_ = dhke.step2_bob(B_, &a)?;
        let C = dhke.step3_alice(C_, r, A)?;

        // Verify the shared secret
        assert!(dhke.verify(a, C, secret_msg)?);
        assert!(!dhke.verify(a, C.combine(&C)?, secret_msg)?); // adding C twice shouldn't pass
        assert!(!dhke.verify(a, A, secret_msg)?); // A shouldn't pass

        Ok(())
    }
}
