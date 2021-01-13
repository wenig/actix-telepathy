/*
def egcd(a, b):
    if a == 0:
        return (b, 0, 1)
    else:
        g, y, x = egcd(b % a, a)
        return (g, x - (b // a) * y, y)

def modinv(a, m):
    g, x, y = egcd(a, m)
    if g != 1:
        raise Exception('modular inverse does not exist')
    else:
        return x % m
 */

use glass_pumpkin::num_bigint::BigInt;
use num_integer::Integer;

pub trait NegModPow {
    fn neg_modpow(&self, exponent: &BigInt, modulus: &BigInt) -> BigInt;
    fn modinv(&self, modulus: &BigInt) -> BigInt;
}

impl NegModPow for BigInt {
    fn neg_modpow(&self, exponent: &BigInt, modulus: &BigInt) -> BigInt {
        let modinv = self.modinv(modulus);
        modinv.modpow(exponent, modulus)
    }

    fn modinv(&self, modulus: &BigInt) -> BigInt {
        let egcd = self.extended_gcd(modulus);

        if egcd.gcd != BigInt::from(1) {
            panic!("modular invers does not exist for your given BigInt")
        } else {
            egcd.x.mod_floor(modulus)
        }
    }
}

