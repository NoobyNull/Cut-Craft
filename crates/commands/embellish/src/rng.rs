//! A small seeded random source (mulberry32), so a seed always makes the same design.

pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// A number from 0 up to 1.
    pub fn f(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(0x3d | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }

    /// A whole number from `a` to `b`, both included.
    pub fn int(&mut self, a: u32, b: u32) -> u32 {
        (a + (self.f() * f64::from(b - a + 1)) as u32).min(b)
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f() < p
    }

    /// One of `list`, each as likely as its weight.
    pub fn pick<T: Copy>(&mut self, list: &[(T, f64)]) -> T {
        let total: f64 = list.iter().map(|m| m.1).sum();
        let mut r = self.f() * total;
        for &(m, w) in list {
            r -= w;
            if r <= 0.0 {
                return m;
            }
        }
        list[list.len() - 1].0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_the_same_numbers_and_they_stay_in_range() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..1000 {
            let x = a.f();
            assert_eq!(x, b.f());
            assert!((0.0..1.0).contains(&x));
            assert!((3..=5).contains(&a.int(3, 5)));
            b.int(3, 5);
        }
        assert_ne!(Rng::new(1).f(), Rng::new(2).f());
    }
}
