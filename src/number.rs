use anyhow::anyhow;
use serde::Deserialize;
use std::{marker::PhantomData, str::FromStr};

pub const MULTIPLIERS: [(char, f64); 3] =
    [('K', 1_000.0), ('M', 1_000_000.0), ('G', 1_000_000_000.0)];

pub fn parse(value: &str) -> anyhow::Result<f64> {
    let mut value = value;
    let mut multiplier = 1.0;
    for (suffix, suffix_multiplier) in MULTIPLIERS {
        if let Some(stripped) = value
            .strip_suffix(suffix)
            .or_else(|| value.strip_suffix(suffix.to_ascii_lowercase()))
        {
            value = stripped;
            multiplier *= suffix_multiplier;
        }
    }
    Ok(value.parse::<f64>()? * multiplier)
}

pub fn write(fmt: &mut std::fmt::Formatter<'_>, value: f64) -> std::fmt::Result {
    if value < 0.0 {
        write!(fmt, "-")?;
    }
    let value = value.abs();
    let mut write_value = |value: f64| -> std::fmt::Result { write!(fmt, "{:.1}", value) };
    for (suffix, suffix_multiplier) in MULTIPLIERS.into_iter().rev() {
        if value > 0.5 * suffix_multiplier {
            write_value(value / suffix_multiplier)?;
            return write!(fmt, "{}", suffix);
        }
    }
    write_value(value)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(f64),
}

pub trait NumberType {
    const PARSE_SUFFIX: bool = true;
    const SUFFIX: Option<&'static str>;
}

#[derive(Deserialize)]
#[serde(try_from = "StringOrNumber")]
pub struct Number<T: NumberType = ()> {
    value: f64,
    phantom_data: PhantomData<T>,
}

impl<T: NumberType> FromStr for Number<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::try_from(StringOrNumber::String(s.into()))
    }
}

impl From<i64> for Number<()> {
    fn from(value: i64) -> Self {
        Self::new(value as f64)
    }
}

impl From<f64> for Number<()> {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl<T: NumberType> Clone for Number<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: NumberType> Copy for Number<T> {}

impl<T: NumberType> Default for Number<T> {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl<T: NumberType> Number<T> {
    pub const fn new(value: f64) -> Self {
        Self {
            value,
            phantom_data: PhantomData,
        }
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn ceil(&self) -> Self {
        Self::new(self.value.ceil())
    }
}

impl<T: NumberType> TryFrom<StringOrNumber> for Number<T> {
    type Error = anyhow::Error;
    fn try_from(value: StringOrNumber) -> Result<Self, Self::Error> {
        let value = match value {
            StringOrNumber::String(value) => {
                let mut value = value.as_str();
                if let Some(suffix) = T::SUFFIX.filter(|_| T::PARSE_SUFFIX) {
                    value = value
                        .strip_suffix(suffix)
                        .ok_or(anyhow!("Value should end with {suffix:?}"))?;
                }
                parse(value)?
            }
            StringOrNumber::Number(number) => {
                if let Some(suffix) = T::SUFFIX.filter(|_| T::PARSE_SUFFIX) {
                    anyhow::bail!("Expected a string with {suffix:?} suffix");
                }
                number
            }
        };
        Ok(Self::new(value))
    }
}

impl<T: NumberType> std::fmt::Debug for Number<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write(f, self.value)?;
        if let Some(suffix) = T::SUFFIX {
            write!(f, "{suffix}")?;
        }
        Ok(())
    }
}

impl<T: NumberType> std::ops::Neg for Number<T> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.value)
    }
}

impl<T: NumberType> std::ops::Add for Number<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.value + rhs.value)
    }
}

impl<T: NumberType> std::ops::AddAssign for Number<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value;
    }
}

impl<T: NumberType> std::ops::Sub for Number<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.value - rhs.value)
    }
}

impl<T: NumberType> std::ops::SubAssign for Number<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.value -= rhs.value;
    }
}

impl<T: NumberType> std::ops::Div for Number<T> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self::new(self.value / rhs.value)
    }
}

impl<T: NumberType> std::ops::DivAssign for Number<T> {
    fn div_assign(&mut self, rhs: Self) {
        self.value /= rhs.value;
    }
}

impl<T: NumberType> std::ops::Mul for Number<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.value * rhs.value)
    }
}

impl<T: NumberType> std::ops::MulAssign for Number<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.value *= rhs.value;
    }
}

impl<T: NumberType> PartialEq for Number<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: NumberType> Eq for Number<T> {}

impl<T: NumberType> PartialOrd for Number<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: NumberType> Ord for Number<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.partial_cmp(&other.value).unwrap()
    }
}
