use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Money(pub u64);

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Money {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul<f32> for Money {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self((self.0 as f32 * rhs).round() as u64)
    }
}

impl Div<u64> for Money {
    type Output = Self;

    fn div(self, rhs: u64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Div<usize> for Money {
    type Output = Self;

    fn div(self, rhs: usize) -> Self::Output {
        Self(self.0 / rhs as u64)
    }
}

impl From<Money> for u64 {
    fn from(m: Money) -> Self {
        m.0
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Money>,
    {
        let sum = iter.fold(0u64, |acc, m| acc + m.0);
        Money(sum)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut value = self.0 as f64;
        let units = ["", "k", "M", "G", "T", "P", "E"];
        let mut unit = "";

        for potential_unit in &units {
            unit = potential_unit;
            if value < 1000.0 {
                break;
            }
            value /= 1000.0;
        }

        let mut string = format!("{:.3}", value);
        string = string.trim_end_matches('0').to_string();
        string = string.trim_end_matches('.').to_string();
        write!(f, "{}{}Cr", string, unit)
    }
}

impl fmt::Debug for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Money {
    pub fn from_string(s: &str) -> Self {
        s.parse::<Money>()
            .unwrap_or_else(|_| panic!("Invalid money format: {}", s))
    }
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl FromStr for Money {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_end_matches(" Cr");
        let (multiplier, len_to_trim) = match s.chars().last().unwrap() {
            'k' => (1_000.0, 1),
            'M' => (1_000_000.0, 1),
            'G' => (1_000_000_000.0, 1),
            'T' => (1_000_000_000_000.0, 1),
            'P' => (1_000_000_000_000_000.0, 1),
            'E' => (1_000_000_000_000_000_000.0, 1),
            _ => (1.0, 0),
        };

        let value_str = &s[..s.len() - len_to_trim];
        match value_str.parse::<f64>() {
            Ok(value) => Ok(Money((value * multiplier) as u64)),
            Err(_) => Err(()),
        }
    }
}
