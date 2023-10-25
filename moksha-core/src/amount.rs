use rand::distributions::Alphanumeric;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Amount(pub u64);

impl Amount {
    pub fn split(&self) -> SplitAmount {
        split_amount(self.0).into()
    }
}

#[derive(Debug, Clone)]
pub struct SplitAmount(pub Vec<u64>);

impl From<Vec<u64>> for SplitAmount {
    fn from(from: Vec<u64>) -> Self {
        Self(from)
    }
}

impl SplitAmount {
    pub fn create_secrets(&self) -> Vec<String> {
        (0..self.0.len())
            .map(|_| generate_random_string())
            .collect::<Vec<String>>()
    }
}

impl From<u64> for Amount {
    fn from(amount: u64) -> Self {
        Self(amount)
    }
}

/// split a decimal amount into a vector of powers of 2
pub fn split_amount(amount: u64) -> Vec<u64> {
    format!("{amount:b}")
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '1' {
                return Some(2_u64.pow(i as u32));
            }
            None
        })
        .collect::<Vec<u64>>()
}

fn generate_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::amount::SplitAmount;

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let bits = super::split_amount(13);
        assert_eq!(bits, vec![1, 4, 8]);

        let bits = super::split_amount(63);
        assert_eq!(bits, vec![1, 2, 4, 8, 16, 32]);

        let bits = super::split_amount(64);
        assert_eq!(bits, vec![64]);
        Ok(())
    }

    #[test]
    fn test_create_secrets() {
        let amounts = vec![1, 2, 3, 4, 5, 6, 7];
        let secrets = SplitAmount::from(amounts.clone()).create_secrets();
        assert!(secrets.len() == amounts.len());
        assert_eq!(secrets.get(0).unwrap().len(), 24);
    }
}
