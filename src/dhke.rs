#![allow(dead_code)]
use bitcoin_hashes::{sha256, Hash};
use secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};

fn get_hash(message: &[u8]) -> Vec<u8> {
    let hash = sha256::Hash::hash(message);
    hash.as_byte_array().to_vec()
}

pub fn hash_to_curve(message: &[u8]) -> PublicKey {
    let mut point: Option<PublicKey> = None;
    let mut msg_to_hash = message.to_vec();
    while point.is_none() {
        let hash = get_hash(&msg_to_hash);
        let input = &[0x02]
            .iter()
            .chain(hash.iter())
            .cloned()
            .collect::<Vec<u8>>();

        match PublicKey::from_slice(input) {
            Ok(p) => point = Some(p),
            Err(_) => msg_to_hash = hash,
        }
    }
    point.unwrap()
}

pub fn step1_alice(
    secret_msg: String,
    blinding_factor: Option<&[u8]>,
) -> Result<(PublicKey, SecretKey), ()> {
    let mut rng = rand::thread_rng();
    let secp = Secp256k1::new();

    let y = hash_to_curve(secret_msg.as_bytes());
    let secret_key = match blinding_factor {
        Some(f) => SecretKey::from_slice(f).unwrap(),
        None => SecretKey::new(&mut rng),
    };
    let b = y
        .combine(&PublicKey::from_secret_key(&secp, &secret_key))
        .unwrap();
    Ok((b, secret_key))
}

fn step2_bob(b: PublicKey, a: SecretKey) -> PublicKey {
    let secp = Secp256k1::new();
    b.mul_tweak(&secp, &Scalar::from(a)).unwrap()
}

// fn step3_alice(C_: PublicKey, r: SecretKey, A: PublicKey) -> PublicKey {
//     let secp = Secp256k1::new();
//     let C: PublicKey = C_.(secp)  - A.mul_tweak(&secp, &Scalar::from(r));
//     return C;
// }

#[cfg(test)]
mod tests {
    use crate::dhke::{hash_to_curve, step1_alice};

    fn hex_to_string(hex: &str) -> String {
        use hex::FromHex;
        let input_vec: Vec<u8> = Vec::from_hex(hex).expect("Invalid Hex String");
        String::from_utf8(input_vec).expect("Invalid UTF-8 String")
    }

    #[test]
    fn test_hash_to_curve_zero() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000000");
        let expected_result = "0266687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925";

        let pk = hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_hash_to_curve_zero_one() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000001");
        let expected_result = "02ec4916dd28fc4c10d78e287ca5d9cc51ee1ae73cbfde08c6b37324cbfaac8bc5";

        let pk = hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_hash_to_curve_iterate() -> anyhow::Result<()> {
        let input_str =
            hex_to_string("0000000000000000000000000000000000000000000000000000000000000002");
        let expected_result = "02076c988b353fcbb748178ecb286bc9d0b4acf474d4ba31ba62334e46c97c416a";

        let pk = hash_to_curve(input_str.as_bytes()).to_string();
        assert_eq!(pk, expected_result);
        Ok(())
    }

    #[test]
    fn test_step1_alice() {
        let pk = step1_alice("test".to_string(), None);
        println!("hash {pk:?}");
    }
}
