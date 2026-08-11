//! Field - 字段
//! 包括 Matrix, Vector, Group, Symbol, Universe 和 Constant
//! Keyword Argument 只能用 Constant, 其余类型互有包含
//! Symbol 和 Universe 主要为了匹配平台枚举, 暂时并未使用

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    Gaussian,
    Uniform,
    Cauchy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    NearestBound,
    Mean,
}

#[derive(Debug, Clone)]
pub enum Constant {
    Float(f64),                                      // 浮点数
    PositiveFloat(f64),                              // 正浮点数
    Ratio(f64),                                      // 比例 (0~1) target_tvr 0.0 ~ 1.0
    Integer(i64),                                    // 整数
    PositiveInteger(u64),                            // 正整数
    Zero,                                            // 0, 0.0
    One,                                             // 1, 1.0
    EnumInteger { range: HashSet<i64>, value: i64 }, // 枚举值
    Boolean(bool),                                   // true / false
    NaN,                                             // "NaN"
    Mask(Mask),                                      // "nearest_bound" / "mean" or numeric values
    Driver(Driver),                                  // "gaussian"
    Range(f64),                                      // 0, 1, 0.1
    Array(Vec<f64>),                                 // buckets, filter(h, t)
    Set(Vec<f64>),                                   // space-separated floats, including NaN
    String(String),
}

#[derive(Debug, Clone)]
pub enum Field {
    Matrix,
    Vector,
    Group,
    Symbol,
    Universe,
    Constant(Constant),
}

impl Constant {
    pub fn from(input: &str) -> Self {
        // trim -> de-quote -> trim -> lowercase
        let input = input.trim();
        let input = if let Some(value) = input
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        {
            value.trim()
        } else if let Some(value) = input
            .strip_prefix('\'')
            .and_then(|value| value.strip_suffix('\''))
        {
            value.trim()
        } else {
            input
        };
        let input = input.to_ascii_lowercase();

        // 字符枚举: Driver / Boolean / NaN / Mask
        match input.as_str() {
            "gaussian" => return Constant::Driver(Driver::Gaussian),
            "uniform" => return Constant::Driver(Driver::Uniform),
            "cauchy" => return Constant::Driver(Driver::Cauchy),
            "true" => return Constant::Boolean(true),
            "false" => return Constant::Boolean(false),
            "nan" => return Constant::NaN,
            "nearest_bound" => return Constant::Mask(Mask::NearestBound),
            "mean" => return Constant::Mask(Mask::Mean),
            _ => {}
        }

        // 数字
        if let Some(constant) = parse_number(&input) {
            return constant;
        }

        // Range 0, 1, 0.1
        if let Some(range) = parse_range(&input) {
            return Constant::Range(range);
        }

        // Array , , , ...
        if let Some(array) = parse_array(&input) {
            return Constant::Array(array);
        }

        if let Some(set) = parse_set(&input) {
            return Constant::Set(set);
        }

        Constant::String(input.to_string())
    }

    pub fn fit(&self, other: &Self) -> bool {
        match (self, other) {
            // Float 是数值类型的宽泛类型，NaN 也属于 Float。
            (Constant::Float(_), actual) => matches!(
                actual,
                Constant::Float(_)
                    | Constant::PositiveFloat(_)
                    | Constant::Ratio(_)
                    | Constant::Integer(_)
                    | Constant::PositiveInteger(_)
                    | Constant::Zero
                    | Constant::One
                    | Constant::EnumInteger { .. }
                    | Constant::NaN
            ),
            // 下面几类需要根据实际值检查范围，而不是只看 enum variant
            (Constant::PositiveFloat(_), actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => value.is_finite() && *value > 0.0,
                Constant::Integer(value) => *value > 0,
                Constant::PositiveInteger(_) | Constant::One => true,
                Constant::EnumInteger { value, .. } => *value > 0,
                _ => false,
            },
            (Constant::Ratio(_), actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => value.is_finite() && (0.0..=1.0).contains(value),
                Constant::Integer(value) => (0..=1).contains(value),
                Constant::PositiveInteger(value) => *value <= 1,
                Constant::Zero | Constant::One => true,
                Constant::EnumInteger { value, .. } => (0..=1).contains(value),
                _ => false,
            },
            (Constant::Integer(_), actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => value.is_finite() && value.fract() == 0.0,
                Constant::Integer(_)
                | Constant::PositiveInteger(_)
                | Constant::Zero
                | Constant::One
                | Constant::EnumInteger { .. } => true,
                _ => false,
            },
            (Constant::PositiveInteger(_), actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => {
                    value.is_finite() && *value > 0.0 && value.fract() == 0.0
                }
                Constant::PositiveInteger(_) | Constant::One => true,
                Constant::Integer(value) => *value > 0,
                Constant::EnumInteger { value, .. } => *value > 0,
                _ => false,
            },
            (Constant::Zero, actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => value.is_finite() && *value == 0.0,
                Constant::Integer(value) | Constant::EnumInteger { value, .. } => *value == 0,
                Constant::Zero => true,
                _ => false,
            },
            (Constant::One, actual) => match actual {
                Constant::Float(value)
                | Constant::PositiveFloat(value)
                | Constant::Ratio(value) => value.is_finite() && *value == 1.0,
                Constant::Integer(value) | Constant::EnumInteger { value, .. } => *value == 1,
                Constant::PositiveInteger(value) => *value == 1,
                Constant::One => true,
                _ => false,
            },
            // Mask accepts its named values, NaN, and any numeric value.
            (Constant::Mask(_), actual) => matches!(
                actual,
                Constant::Mask(_)
                    | Constant::Float(_)
                    | Constant::PositiveFloat(_)
                    | Constant::Ratio(_)
                    | Constant::Integer(_)
                    | Constant::PositiveInteger(_)
                    | Constant::Zero
                    | Constant::One
                    | Constant::EnumInteger { .. }
                    | Constant::NaN
            ),

            // EnumInteger 是集合类型：实际集合必须是期望集合的子集。
            (Constant::EnumInteger { range, value: _ }, Constant::EnumInteger { value, .. }) => {
                range.contains(value)
            }
            (Constant::EnumInteger { range, value: _ }, Constant::Integer(value)) => {
                range.contains(value)
            }
            (Constant::EnumInteger { range, value: _ }, Constant::PositiveInteger(value)) => {
                i64::try_from(*value).is_ok_and(|value| range.contains(&value))
            }
            (Constant::EnumInteger { range, value: _ }, Constant::Zero) => range.contains(&0),
            (Constant::EnumInteger { range, value: _ }, Constant::One) => range.contains(&1),
            (Constant::EnumInteger { range, value: _ }, Constant::Float(value))
            | (Constant::EnumInteger { range, value: _ }, Constant::PositiveFloat(value))
            | (Constant::EnumInteger { range, value: _ }, Constant::Ratio(value)) => {
                value.is_finite()
                    && value.fract() == 0.0
                    && *value >= i64::MIN as f64
                    && *value <= i64::MAX as f64
                    && range.contains(&(*value as i64))
            }

            (Constant::Boolean(_), Constant::Boolean(_))
            | (Constant::NaN, Constant::NaN)
            | (Constant::Driver(_), Constant::Driver(_))
            | (Constant::Range(_), Constant::Range(_))
            | (Constant::Array(_), Constant::Array(_))
            | (Constant::Array(_), Constant::Range(_))
            // Array 参数也接受单个数值常量。
            | (Constant::Array(_), Constant::Float(_))
            | (Constant::Array(_), Constant::PositiveFloat(_))
            | (Constant::Array(_), Constant::Ratio(_))
            | (Constant::Array(_), Constant::Integer(_))
            | (Constant::Array(_), Constant::PositiveInteger(_))
            | (Constant::Array(_), Constant::Zero)
            | (Constant::Array(_), Constant::One)
            | (Constant::Array(_), Constant::EnumInteger { .. })
            | (Constant::Set(_), Constant::Set(_))
            | (Constant::String(_), Constant::String(_)) => true,
            _ => false,
        }
    }
}

impl Field {
    pub fn fit(&self, other: &Self) -> bool {
        match (self, other) {
            (Field::Matrix, Field::Matrix)
            | (Field::Vector, Field::Vector)
            | (Field::Group, Field::Group)
            | (Field::Symbol, Field::Symbol)
            | (Field::Universe, Field::Universe) => true,
            (Field::Matrix, Field::Constant(constant)) => {
                matches!(
                    constant,
                    Constant::Float(_)
                        | Constant::PositiveFloat(_)
                        | Constant::Ratio(_)
                        | Constant::Integer(_)
                        | Constant::PositiveInteger(_)
                        | Constant::Zero
                        | Constant::One
                        | Constant::EnumInteger { .. }
                        | Constant::NaN
                )
            }
            (Field::Constant(expected), Field::Constant(actual)) => expected.fit(actual),
            _ => false,
        }
    }
}

fn parse_number(input: &str) -> Option<Constant> {
    // 尝试解析整数
    // 0   -> NonNegativeInteger
    // 1   ->
    // >0  -> PositiveInteger
    // _   -> Integer
    if let Ok(value) = input.parse::<i64>() {
        return Some(if value == 0 {
            Constant::Zero
        } else if value == 1 {
            Constant::One
        } else if value > 0 {
            Constant::PositiveInteger(value as u64)
        } else {
            Constant::Integer(value)
        });
    }

    // 尝试解析浮点数
    let value = input.parse::<f64>().ok()?;
    if value.is_nan() {
        return Some(Constant::NaN);
    } else if value.is_infinite() {
        return None;
    }

    // 浮点数转整数
    if value.fract() == 0.0 {
        if value == 0.0 {
            return Some(Constant::Zero);
        } else if value == 1.0 {
            return Some(Constant::One);
        } else if value > 0.0 && value <= u64::MAX as f64 {
            return Some(Constant::PositiveInteger(value as u64));
        } else if value < 0.0 && value >= i64::MIN as f64 {
            return Some(Constant::Integer(value as i64));
        }
    }
    // Ratio
    if (0.0..=1.0).contains(&value) {
        return Some(Constant::Ratio(value));
    }

    Some(if value > 0.0 {
        Constant::PositiveFloat(value)
    } else {
        Constant::Float(value)
    })
}

fn parse_range(input: &str) -> Option<f64> {
    let mut parts = input.split(',');
    if parts.next()?.trim() != "0" || parts.next()?.trim() != "1" {
        return None;
    }

    let step = parts.next()?.trim().parse::<f64>().ok()?;
    if parts.next().is_some() || !(step > 0.0 && step < 1.0) {
        return None;
    }

    Some(step)
}

fn parse_float(input: &str) -> Option<f64> {
    if input.trim().eq_ignore_ascii_case("nan") {
        Some(f64::NAN)
    } else {
        input.trim().parse::<f64>().ok()
    }
}

fn parse_array(input: &str) -> Option<Vec<f64>> {
    if !input.contains(',') {
        return None;
    }

    let values: Option<Vec<_>> = input.split(',').map(parse_float).collect();
    let values = values?;
    (!values.is_empty()).then_some(values)
}

fn parse_set(input: &str) -> Option<Vec<f64>> {
    let parts: Vec<_> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    parts.into_iter().map(parse_float).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{Constant, Driver, Field, Mask};

    #[test]
    fn parses_integer_categories() {
        assert!(matches!(
            Constant::from(" 42 "),
            Constant::PositiveInteger(42)
        ));
        assert!(matches!(Constant::from("-42"), Constant::Integer(-42)));
        assert!(matches!(Constant::from("0"), Constant::Zero));
        assert!(matches!(Constant::from("0.0"), Constant::Zero));
        assert!(matches!(Constant::from("1"), Constant::One));
        assert!(matches!(Constant::from("1.0"), Constant::One));
    }

    #[test]
    fn parses_float_categories() {
        match Constant::from("0.25") {
            Constant::Ratio(value) => assert_eq!(value, 0.25),
            other => panic!("expected Ratio, got {other:?}"),
        }

        match Constant::from("2.5") {
            Constant::PositiveFloat(value) => assert_eq!(value, 2.5),
            other => panic!("expected PositiveFloat, got {other:?}"),
        }

        match Constant::from("-2.5") {
            Constant::Float(value) => assert_eq!(value, -2.5),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn parses_case_insensitive_named_constants_with_quotes() {
        assert!(matches!(
            Constant::from(" \"GAUSSIAN\" "),
            Constant::Driver(Driver::Gaussian)
        ));
        assert!(matches!(Constant::from("'TrUe'"), Constant::Boolean(true)));
        assert!(matches!(Constant::from("NaN"), Constant::NaN));
        assert!(matches!(Constant::from("'nan'"), Constant::NaN));
    }

    #[test]
    fn parses_mask_names_case_insensitively() {
        assert!(matches!(
            Constant::from("'NEAREST_BOUND'"),
            Constant::Mask(Mask::NearestBound)
        ));
        assert!(matches!(Constant::from("mean"), Constant::Mask(Mask::Mean)));
    }

    #[test]
    fn mask_accepts_nan_and_all_numeric_constants() {
        let expected = Constant::Mask(Mask::Mean);
        for actual in [
            Constant::NaN,
            Constant::Float(-1.5),
            Constant::PositiveFloat(2.5),
            Constant::Ratio(0.5),
            Constant::Integer(-2),
            Constant::PositiveInteger(2),
            Constant::Zero,
            Constant::One,
            Constant::EnumInteger {
                range: HashSet::from([1, 2]),
                value: 1,
            },
        ] {
            assert!(expected.fit(&actual));
        }
        assert!(expected.fit(&Constant::Mask(Mask::NearestBound)));
        assert!(!expected.fit(&Constant::Boolean(true)));
        assert!(!expected.fit(&Constant::String("other".into())));
    }

    #[test]
    fn parses_range_and_array_while_ignoring_separator_whitespace() {
        match Constant::from(" 0, 1, 0.1 ") {
            Constant::Range(step) => assert_eq!(step, 0.1),
            other => panic!("expected Range, got {other:?}"),
        }

        match Constant::from(" 1, nan, 3 ") {
            Constant::Array(values) => {
                assert_eq!(values.len(), 3);
                assert_eq!(values[0], 1.0);
                assert!(values[1].is_nan());
                assert_eq!(values[2], 3.0);
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }

    #[test]
    fn parses_space_separated_set_with_nan() {
        match Constant::from("0 1 nan 2.5 ") {
            Constant::Set(values) => {
                assert_eq!(values.len(), 4);
                assert_eq!(values[0], 0.0);
                assert_eq!(values[1], 1.0);
                assert!(values[2].is_nan());
                assert_eq!(values[3], 2.5);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_string_or_array() {
        assert!(matches!(
            Constant::from(" hello world "),
            Constant::String(value) if value == "hello world"
        ));
        assert!(matches!(
            Constant::from("0, 1, 1"),
            Constant::Array(values) if values == vec![0.0, 1.0, 1.0]
        ));
    }

    #[test]
    fn matches_field_types() {
        assert!(Field::Matrix.fit(&Field::Matrix));
        assert!(Field::Matrix.fit(&Field::Constant(Constant::Float(1.5))));
        assert!(Field::Matrix.fit(&Field::Constant(Constant::NaN)));
        assert!(!Field::Matrix.fit(&Field::Constant(Constant::Boolean(true))));
        assert!(Field::Vector.fit(&Field::Vector));
        assert!(!Field::Vector.fit(&Field::Matrix));
        assert!(Field::Group.fit(&Field::Group));
        assert!(Field::Symbol.fit(&Field::Symbol));
        assert!(Field::Universe.fit(&Field::Universe));
    }

    #[test]
    fn matches_constant_inclusion_rules() {
        assert!(Constant::Float(0.0).fit(&Constant::Ratio(0.5)));
        assert!(Constant::Float(0.0).fit(&Constant::PositiveInteger(3)));
        assert!(Constant::Float(0.0).fit(&Constant::NaN));
        assert!(!Constant::Float(0.0).fit(&Constant::Boolean(true)));

        assert!(Constant::PositiveInteger(0).fit(&Constant::One));
        assert!(Constant::PositiveInteger(0).fit(&Constant::Float(3.0)));
        assert!(!Constant::PositiveInteger(0).fit(&Constant::Zero));
        assert!(Constant::Ratio(0.0).fit(&Constant::Zero));
        assert!(Constant::Ratio(0.0).fit(&Constant::One));
        assert!(!Constant::Ratio(0.0).fit(&Constant::PositiveInteger(2)));
        assert!(Constant::Array(Vec::new()).fit(&Constant::Range(0.1)));
        assert!(Constant::Array(Vec::new()).fit(&Constant::Float(0.5)));
        assert!(Constant::Array(Vec::new()).fit(&Constant::PositiveInteger(1)));
        assert!(!Constant::Array(Vec::new()).fit(&Constant::Boolean(true)));
        assert!(!Constant::Array(Vec::new()).fit(&Constant::String("x".into())));
        assert!(!Constant::Range(0.1).fit(&Constant::Array(Vec::new())));

        let expected = HashSet::from([1, 2, 3]);
        let actual = Constant::EnumInteger {
            range: HashSet::from([1, 3]),
            value: 3,
        };
        assert!(
            Constant::EnumInteger {
                range: expected,
                value: 1
            }
            .fit(&actual)
        );
    }
}
