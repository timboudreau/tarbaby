use rand::{Rng as _, rngs::ThreadRng};
use std::path::Path;

const ALPHABET: [u8; 52] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const OUTPUT_SIZE: usize = 2048;

pub fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("random_alphanumeric.txt");

    if dest_path.exists() {
        let n = std::fs::metadata(&dest_path).unwrap();
        if n.len() == OUTPUT_SIZE as u64 {
            return;
        }
    }

    let mut v = Vec::new();
    random_string(OUTPUT_SIZE, &mut v);

    std::fs::write(&dest_path, &v).expect("Failed to write random chars");
    println!("cargo::warning=Wrote random chars to {:?}", dest_path);
    println!("cargo::rerun-if-changed=build.rs");
}

fn random_string(n: usize, result: &mut Vec<u8>) {
    let mut rng = rand::rng();
    for _ in 0..n {
        result.push(random_char(&mut rng));
    }
}

fn random_char(rng: &mut ThreadRng) -> u8 {
    let ix: usize = (rng.next_u32() as usize) % ALPHABET.len();
    ALPHABET[ix]
}
