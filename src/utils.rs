use rand::{distr::Alphanumeric, Rng};

pub fn gen_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}
