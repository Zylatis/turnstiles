use std::num::Wrapping;
type u64_w = Wrapping<u64>;

const state: u64 = 0x4d595df4d0f33173;
const multiplier: u64 = 6364136223846793005;
const increment: u64 = 1442695040888963407;

#[derive(Default, Debug)]
pub struct PCG32Rand {
    state: u64,
}

fn rotr32(x: u32, r: u32) -> u32 {
    return x >> r | x << (-(r as i64) & 31);
}

impl PCG32Rand {
    pub fn new(seed: u64) -> Self {
        PCG32Rand {
            state: (Wrapping(state) + Wrapping(seed)).0,
        }
    }

    pub fn rand(&mut self) -> u32 {
        let mut x: u64 = self.state;
        let count: u64 = x >> 59;
        dbg!(x, count);
        self.state = (Wrapping(x) * Wrapping(multiplier) + Wrapping(increment)).0;
        x ^= x >> 18;
        return rotr32((x >> 27) as u32, count as u32);
    }
}

mod tests {
    #![allow(arithmetic_overflow)]

    use super::PCG32Rand;

    #[test]
    fn test_rng() {
        let mut rng = PCG32Rand::new(1);
        let a = rng.rand();
        let b = rng.rand();
        assert_ne!(a, b);
    }
    #[test]
    fn test_rng_seed() {
        let mut rng = PCG32Rand::new(1);
        let mut rng_2 = PCG32Rand::new(10);
        dbg!(&rng);
        dbg!(&rng_2);
        assert_ne!(rng.rand(), rng_2.rand());
    }
}
