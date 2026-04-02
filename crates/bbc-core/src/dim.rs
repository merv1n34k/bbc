/// Packed dimension vector -- 7 x i8 exponents in a single u64.
///
/// Layout: [m:8][kg:8][s:8][A:8][K:8][mol:8][cd:8][unused:8]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct DimVec(pub u64);

impl DimVec {
    pub const DIMENSIONLESS: DimVec = DimVec(0);

    pub const M: usize = 0;
    pub const KG: usize = 1;
    pub const S: usize = 2;
    pub const A: usize = 3;
    pub const K: usize = 4;
    pub const MOL: usize = 5;
    pub const CD: usize = 6;

    pub fn new(exponents: [i8; 7]) -> Self {
        let mut v: u64 = 0;
        for (i, &e) in exponents.iter().enumerate() {
            v |= ((e as u8) as u64) << ((6 - i) * 8);
        }
        DimVec(v)
    }

    pub fn get(self, idx: usize) -> i8 {
        debug_assert!(idx < 7);
        ((self.0 >> ((6 - idx) * 8)) & 0xFF) as u8 as i8
    }

    pub fn mul(self, other: DimVec) -> DimVec {
        let mut result = [0i8; 7];
        for i in 0..7 {
            result[i] = self.get(i).wrapping_add(other.get(i));
        }
        DimVec::new(result)
    }

    pub fn div(self, other: DimVec) -> DimVec {
        let mut result = [0i8; 7];
        for i in 0..7 {
            result[i] = self.get(i).wrapping_sub(other.get(i));
        }
        DimVec::new(result)
    }

    pub fn pow(self, n: i8) -> DimVec {
        let mut result = [0i8; 7];
        for i in 0..7 {
            result[i] = self.get(i).wrapping_mul(n);
        }
        DimVec::new(result)
    }

    pub fn root(self, n: i8) -> Option<DimVec> {
        let mut result = [0i8; 7];
        for i in 0..7 {
            let e = self.get(i);
            if e % n != 0 {
                return None;
            }
            result[i] = e / n;
        }
        Some(DimVec::new(result))
    }

    pub fn is_dimensionless(self) -> bool {
        self.0 == 0
    }

    pub fn compatible(self, other: DimVec) -> bool {
        self.0 == other.0
    }

    pub fn to_array(self) -> [i8; 7] {
        let mut arr = [0i8; 7];
        for i in 0..7 {
            arr[i] = self.get(i);
        }
        arr
    }
}

impl std::fmt::Debug for DimVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for DimVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = ["m", "kg", "s", "A", "K", "mol", "cd"];
        let mut parts = Vec::new();
        for i in 0..7 {
            let e = self.get(i);
            if e != 0 {
                if e == 1 {
                    parts.push(names[i].to_string());
                } else {
                    parts.push(format!("{}^{}", names[i], e));
                }
            }
        }
        if parts.is_empty() {
            write!(f, "dimensionless")
        } else {
            write!(f, "{}", parts.join("*"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensionless() {
        let d = DimVec::DIMENSIONLESS;
        assert!(d.is_dimensionless());
        assert_eq!(d.to_array(), [0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn velocity() {
        let v = DimVec::new([1, 0, -1, 0, 0, 0, 0]);
        assert_eq!(v.get(DimVec::M), 1);
        assert_eq!(v.get(DimVec::S), -1);
        assert!(!v.is_dimensionless());
    }

    #[test]
    fn mul_div() {
        let m = DimVec::new([1, 0, 0, 0, 0, 0, 0]);
        let s = DimVec::new([0, 0, 1, 0, 0, 0, 0]);
        let velocity = m.div(s);
        assert_eq!(velocity, DimVec::new([1, 0, -1, 0, 0, 0, 0]));
        let accel = velocity.div(s);
        assert_eq!(accel, DimVec::new([1, 0, -2, 0, 0, 0, 0]));
    }

    #[test]
    fn pow_and_root() {
        let m = DimVec::new([1, 0, 0, 0, 0, 0, 0]);
        let m2 = m.pow(2);
        assert_eq!(m2, DimVec::new([2, 0, 0, 0, 0, 0, 0]));
        assert_eq!(m2.root(2), Some(m));
        let m3 = m.pow(3);
        assert_eq!(m3.root(2), None);
    }

    #[test]
    fn compatibility() {
        let a = DimVec::new([1, 0, -2, 0, 0, 0, 0]);
        let b = DimVec::new([1, 0, -2, 0, 0, 0, 0]);
        let c = DimVec::new([0, 0, -2, 0, 0, 0, 0]);
        assert!(a.compatible(b));
        assert!(!a.compatible(c));
    }

    #[test]
    fn display() {
        let force = DimVec::new([1, 1, -2, 0, 0, 0, 0]);
        assert_eq!(format!("{}", force), "m*kg*s^-2");
    }
}
