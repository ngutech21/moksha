#![allow(dead_code)]
use bitcoin_hashes::{sha256, Hash};
use secp256k1::{Error, PublicKey, Scalar, Secp256k1, SecretKey};

pub fn hash_to_curve(message: String) -> Result<PublicKey, Error> {
    let mut point = None;
    let mut msg_to_hash = message;
    while point.is_none() {
        let hash = sha256::Hash::hash(msg_to_hash.as_bytes());
        let hash_array = hash.as_byte_array();

        let input = &[0x02]
            .iter()
            .chain(hash_array.iter())
            .cloned()
            .collect::<Vec<u8>>();

        point = PublicKey::from_slice(input).ok();
        msg_to_hash = hash.to_string();
    }
    Ok(point.unwrap())
}

pub fn step1_alice(
    secret_msg: String,
    blinding_factor: Option<&[u8]>,
) -> Result<(PublicKey, SecretKey), ()> {
    let mut rng = rand::thread_rng();
    let secp = Secp256k1::new();

    let y = hash_to_curve(secret_msg).unwrap();
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

    #[test]
    fn test_hash_to_curve() {
        // let sh = sha256::Hash::hash("test".as_bytes());
        // let b = sh.as_byte_array();
        // println!("sh {b:?}");

        // let pubkey = PublicKey::from_slice(&[
        //     2, 29, 21, 35, 7, 198, 183, 43, 14, 208, 65, 139, 14, 112, 205, 128, 231, 245, 41, 91, 141,
        //     134, 245, 114, 45, 63, 82, 19, 251, 210, 57, 79, 54,
        // ])
        // .unwrap();
        // println!(">{:?}<", pubkey);
        let pk = hash_to_curve("test".to_string());
        println!("hash {pk:?}");
    }

    #[test]
    fn test_step1_alice() {
        let pk = step1_alice("test".to_string(), None);
        println!("hash {pk:?}");
    }
}
