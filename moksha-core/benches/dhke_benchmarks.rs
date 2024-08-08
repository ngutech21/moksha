use criterion::{criterion_group, criterion_main, Criterion};
use moksha_core::dhke::Dhke;
use secp256k1::{Secp256k1, SecretKey};

fn bench_dhke(c: &mut Criterion) {
    let secp = Secp256k1::new();
    let dhke = Dhke::new();
    let secret_msg = "test_message";
    let a = SecretKey::from_slice(&[1; 32]).unwrap();
    let blinding_factor = SecretKey::from_slice(&[1; 32]).unwrap();

    c.bench_function("hashToPoint", |b| {
        b.iter(|| Dhke::hash_to_curve(secret_msg.as_bytes()).unwrap())
    });

    c.bench_function("step1Alice", |b| {
        b.iter(|| {
            dhke.step1_alice(secret_msg, &blinding_factor.into())
                .unwrap()
        })
    });

    let b_ = dhke
        .step1_alice(secret_msg, &blinding_factor.into())
        .unwrap();
    c.bench_function("step2Bob", |b| b.iter(|| dhke.step2_bob(b_, &a).unwrap()));

    let c_ = dhke.step2_bob(b_, &a).unwrap();
    c.bench_function("step3Alice", |b| {
        b.iter(|| {
            dhke.step3_alice(c_, blinding_factor.into(), a.public_key(&secp))
                .unwrap()
        })
    });

    let step3_c = dhke
        .step3_alice(c_, blinding_factor.into(), a.public_key(&secp))
        .unwrap();
    c.bench_function("verify", |b| {
        b.iter(|| dhke.verify(a, step3_c, secret_msg).unwrap())
    });

    c.bench_function("End-to-End BDHKE", |b| {
        b.iter(|| {
            let b_ = dhke
                .step1_alice(secret_msg, &blinding_factor.into())
                .unwrap();
            let c_ = dhke.step2_bob(b_, &a).unwrap();
            let c = dhke
                .step3_alice(c_, blinding_factor.into(), a.public_key(&secp))
                .unwrap();
            dhke.verify(a, c, secret_msg).unwrap()
        })
    });
}

criterion_group!(benches, bench_dhke);
criterion_main!(benches);
