use log::*;


pub struct OneFactor {
    n_peers: i32,
    i: i32,
    j: i32
}

impl OneFactor {
    pub fn new(n_peers: usize) -> Self {
        Self{
            n_peers: n_peers as i32,
            i: 0,
            j: -1
        }
    }

    fn iteration(&mut self, i_max: i32, j_max: i32, even: bool) -> Option<OneFactorCouple>
    {
        if self.j < j_max {
            self.j = self.j + 1;
            if even {
                self.even_sized()
            } else {
                self.odd_sized()
            }
        } else if self.i < i_max {
            self.i = self.i + 1;
            self.j = 0;
            if even {
                self.even_sized()
            } else {
                self.odd_sized()
            }
        } else {
            None
        }
    }

    fn odd_sized(&mut self) -> Option<OneFactorCouple> {
        let partner = (self.i-self.j).rem_euclid(self.n_peers);
        if partner == self.j {
            return self.next()
        }
        Some(OneFactorCouple::new(self.j, partner))
    }

    fn even_sized(&mut self) -> Option<OneFactorCouple> {
        let idle = ((self.n_peers / 2) * self.i).rem_euclid(self.n_peers - 1);
        if self.j == (self.n_peers - 1) {
            Some(OneFactorCouple::new(self.j, idle))
        } else {
            if self.j == idle {
                Some(OneFactorCouple::new(self.j, self.n_peers - 1))
            } else {
                let partner = (self.i - self.j).rem_euclid(self.n_peers - 1);
                Some(OneFactorCouple::new(self.j, partner))
            }
        }
    }
}

pub struct OneFactorCouple {
    pub active: usize,
    pub passive: usize
}

impl OneFactorCouple {
    pub fn new(active: i32, passive: i32) -> Self {
        Self {active: active as usize, passive: passive as usize}
    }
}

impl Iterator for OneFactor {
    type Item = OneFactorCouple;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n_peers % 2 == 0 {
            self.iteration(self.n_peers - 2, self.n_peers - 1, true)
        } else {
            self.iteration(self.n_peers - 1, self.n_peers - 1, false)
        }
    }
}


// ----------------------------
//            TESTS
// ----------------------------

#[cfg(test)]
mod tests {
    use crate::security::protocols::mascot::one_factor::{OneFactor, OneFactorCouple};
    use std::collections::HashSet;

    fn count_pairs(length: usize) -> usize {
        let one_factor = OneFactor::new(length);
        let mut pair_set: HashSet<(usize, usize)> = HashSet::new();

        for couple in one_factor {
            pair_set.insert((couple.active, couple.passive));
        }
        pair_set.len()
    }

    #[test]
    fn no_double_assignment_even() {
        let length = 6;
        assert_eq!(count_pairs(length), length * (length - 1));
    }

    #[test]
    fn no_double_assignment_odd() {
        let length = 7;
        assert_eq!(count_pairs(length), length * (length - 1));
    }

    #[test]
    fn no_self_connection_even() {
        let length = 6;

        let one_factor = OneFactor::new(length);

        for couple in one_factor {
            assert_ne!(couple.active, couple.passive);
        }
    }

    #[test]
    fn no_self_connection_odd() {
        let length = 7;

        let one_factor = OneFactor::new(length);

        for couple in one_factor {
            assert_ne!(couple.active, couple.passive);
        }
    }
}
