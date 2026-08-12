use crate::field::{Constant, Driver, Field, Mask};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: &'static str,
    pub param_type: Field,
    pub default_value: Option<Constant>,
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub name: &'static str,
    pub pos_args: Vec<Field>,
    pub kw_args: HashMap<String, ParamSpec>,
    pub nary: i32,
    pub return_type: Field,
    pub description: &'static str,
}

#[derive(Debug)]
pub enum OperatorCheckError {
    UnknownOperator(String),
    InvalidPositionalArgumentCount {
        expected: usize,
        actual: usize,
    },
    NaryPosArgsLessThanTwo,
    InvalidArgumentType {
        expected: Box<Field>,
        actual: Box<Field>,
    },
    UnknownKeyword(String),
    MissingKeyword(String),
    InvalidKeywordType {
        name: String,
        expected: Box<Field>,
        actual: Box<Field>,
    },
    UnknownVariable(String),
    UnresolvedPlaceholder(String),
    InvalidExpression(String),
}

impl Operator {
    pub fn apply(
        &self,
        pos_args: &[Field],
        kw_args: &HashMap<String, Constant>,
    ) -> Result<Field, OperatorCheckError> {
        // PosArgs 数量预检
        match (self.nary, pos_args.len()) {
            (-1, actual) if actual < 2 => {
                return Err(OperatorCheckError::NaryPosArgsLessThanTwo);
            }
            (expect, actual) if expect > 0 && expect as usize != actual => {
                return Err(OperatorCheckError::InvalidPositionalArgumentCount {
                    expected: expect as usize,
                    actual,
                });
            }
            _ => {}
        }
        // PosArgs 类型检测
        for (index, actual) in pos_args.iter().enumerate() {
            let expected = if self.nary == -1 {
                &self.pos_args[0]
            } else {
                &self.pos_args[index]
            };

            if !expected.fit(actual) {
                return Err(OperatorCheckError::InvalidArgumentType {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(actual.clone()),
                });
            }
        }
        // KwArgs 类型检测
        for (name, value) in kw_args {
            let Some(spec) = self.kw_args.get(name) else {
                return Err(OperatorCheckError::UnknownKeyword(name.clone()));
            };

            let actual = Field::Constant(value.clone());
            if !spec.param_type.fit(&actual) {
                return Err(OperatorCheckError::InvalidKeywordType {
                    name: name.clone(),
                    expected: Box::new(spec.param_type.clone()),
                    actual: Box::new(actual),
                });
            }
        }

        for spec in self.kw_args.values() {
            if !kw_args.contains_key(spec.name) && spec.default_value.is_none() {
                return Err(OperatorCheckError::MissingKeyword(spec.name.to_string()));
            }
        }

        self.validate_special_constraints(pos_args, kw_args)?;

        Ok(self.return_type.clone())
    }

    fn validate_special_constraints(
        &self,
        pos_args: &[Field],
        kw_args: &HashMap<String, Constant>,
    ) -> Result<(), OperatorCheckError> {
        match self.name {
            "kth_element" => {
                let d = positive_integer(pos_args.get(1));
                let k = kw_args.get("k").map(|value| Field::Constant(value.clone()));
                if let (Some(d), Some(k)) = (d, positive_integer(k.as_ref()))
                    && k > d
                {
                    return Err(OperatorCheckError::InvalidExpression(
                        "kth_element requires k <= d".to_string(),
                    ));
                }
            }
            "ts_backfill" => {
                let d = positive_integer(pos_args.get(1));
                let k = kw_args.get("k").or_else(|| {
                    self.kw_args
                        .get("k")
                        .and_then(|spec| spec.default_value.as_ref())
                });
                let k = k.map(|k| Field::Constant(k.clone()));
                if let (Some(d), Some(k)) = (d, positive_integer(k.as_ref()))
                    && k > d
                {
                    return Err(OperatorCheckError::InvalidExpression(
                        "ts_backfill requires k <= d".to_string(),
                    ));
                }
            }
            "bucket" => {
                let range = kw_args.get("range").or_else(|| {
                    self.kw_args
                        .get("range")
                        .and_then(|spec| spec.default_value.as_ref())
                });
                let buckets = kw_args.get("buckets").or_else(|| {
                    self.kw_args
                        .get("buckets")
                        .and_then(|spec| spec.default_value.as_ref())
                });
                if let Some(buckets) = buckets
                    && !is_nan_constant(buckets)
                    && matches!(buckets, Constant::Range(_))
                {
                    return Err(OperatorCheckError::InvalidExpression(
                        "bucket requires buckets to be an Array, not a Range, consider use the range param".to_string(),
                    ));
                }
                if let Some(Constant::Array(values)) = buckets
                    && !values
                        .windows(2)
                        .all(|pair| pair[0].is_finite() && pair[1].is_finite() && pair[1] > pair[0])
                {
                    return Err(OperatorCheckError::InvalidExpression(
                        "bucket requires buckets to be strictly increasing".to_string(),
                    ));
                }
                if let (Some(range), Some(buckets)) = (range, buckets) {
                    let range_is_nan = is_nan_constant(range);
                    let buckets_is_nan = is_nan_constant(buckets);
                    if range_is_nan == buckets_is_nan {
                        return Err(OperatorCheckError::InvalidExpression(
                            "bucket requires exactly one of range and buckets to be valid"
                                .to_string(),
                        ));
                    }
                }
            }
            "clamp" => {
                let lower = kw_args.get("lower").and_then(number_value);
                let upper = kw_args.get("upper").and_then(number_value);
                if let (Some(lower), Some(upper)) = (lower, upper)
                    && upper <= lower
                {
                    return Err(OperatorCheckError::InvalidExpression(
                        "clamp requires upper > lower".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn positive_integer(field: Option<&Field>) -> Option<u64> {
    let Some(Field::Constant(value)) = field else {
        return None;
    };
    let value = number_value(value)?;
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u64::MAX as f64)
        .then_some(value as u64)
}

fn number_value(value: &Constant) -> Option<f64> {
    match value {
        Constant::Float(value) | Constant::PositiveFloat(value) | Constant::Ratio(value) => {
            Some(*value)
        }
        Constant::Integer(value) => Some(*value as f64),
        Constant::PositiveInteger(value) => Some(*value as f64),
        Constant::Zero => Some(0.0),
        Constant::One => Some(1.0),
        Constant::EnumInteger { value, .. } => Some(*value as f64),
        _ => None,
    }
}

fn is_nan_constant(value: &Constant) -> bool {
    matches!(value, Constant::NaN)
        || matches!(value, Constant::Range(value) if value.is_nan())
        || matches!(value, Constant::Array(values) if values.len() == 1 && values[0].is_nan())
}

macro_rules! param_type {
    (Matrix) => {
        Field::Matrix
    };
    (Vector) => {
        Field::Vector
    };
    (Group) => {
        Field::Group
    };
    (Boolean) => {
        Field::Constant(Constant::Boolean(false))
    };
    (Number) => {
        Field::Constant(Constant::Float(0.0))
    };
    (Mask) => {
        Field::Constant(Constant::Mask(Mask::NearestBound))
    };
    (PositiveInt) => {
        Field::Constant(Constant::PositiveInteger(1))
    };
    (Int) => {
        Field::Constant(Constant::Integer(0))
    };
    (NonNegativeInt) => {
        Field::Constant(Constant::NonNegativeInteger(0))
    };
    (PositiveFloat) => {
        Field::Constant(Constant::PositiveFloat(0.1))
    };
    (Range) => {
        Field::Constant(Constant::Range(0.1))
    };
    (Array) => {
        Field::Constant(Constant::Array(Vec::new()))
    };
    (NonNegativeFloat) => {
        Field::Constant(Constant::NonNegativeFloat(0.0))
    };
    (Ratio) => {
        Field::Constant(Constant::Ratio(0.1))
    };
    (String) => {
        Field::Constant(Constant::String(String::from(" ")))
    };
    (Driver) => {
        Field::Constant(Constant::Driver(Driver::Gaussian))
    };
    (Constant) => {
        Field::Constant(Constant::String(String::new()))
    };
    (Set) => {
        Field::Constant(Constant::Set(Vec::new()))
    };
    (RetType) => {
        Field::Constant(Constant::EnumInteger {
            range: HashSet::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            value: 0,
        })
    };
    (ReturnMode) => {
        Field::Constant(Constant::EnumInteger {
            range: HashSet::from([1, 2]),
            value: 1,
        })
    };
    (RankRate) => {
        Field::Constant(Constant::EnumInteger {
            range: HashSet::from([0, 2]),
            value: 2,
        })
    };
    ($constant_type:ident) => {
        compile_error!(concat!(
            "unsupported parameter type: ",
            stringify!($constant_type)
        ))
    };
}

macro_rules! define_operator {
    (
        $static_name:ident,
        $name:literal,
        [$($pos_arg:ident),* $(,)?],
        {$($kw_name:literal => ($kw_type:ident, $default:expr)),* $(,)?},
        $nary:expr,
        $return_type:ident,
        $description:literal $(,)?
    ) => {
        pub static $static_name: LazyLock<Operator> = LazyLock::new(|| Operator {
                name: $name,
                pos_args: vec![$(param_type!($pos_arg)),*],
                kw_args: HashMap::from([
                    $(($kw_name.to_owned(), ParamSpec {
                        name: $kw_name,
                        param_type: param_type!($kw_type),
                        default_value: $default,
                    })),*
                ]),
                nary: $nary,
                return_type: param_type!($return_type),
                description: $description,
            });
    };
}

macro_rules! define_operator_registry {
    ($(define_operator!($static_name:ident, $($operator:tt)*);)*) => {
        $(define_operator!($static_name, $($operator)*);)*

        pub static OPERATORS: &[&LazyLock<Operator>] = &[
            $(&$static_name),*
        ];

        static OPERATOR_REGISTRY: LazyLock<HashMap<&'static str, &'static Operator>> =
            LazyLock::new(|| {
                let mut registry = HashMap::with_capacity(OPERATORS.len());

                for lazy_operator in OPERATORS {
                    let operator: &'static Operator = &**lazy_operator;
                    assert!(
                        registry.insert(operator.name, operator).is_none(),
                        "duplicate operator name: {}",
                        operator.name,
                    );
                }

                registry
            });

        pub fn get_operator(name: &str) -> Option<&'static Operator> {
            OPERATOR_REGISTRY.get(name).copied()
        }
    };
}

define_operator_registry! {
    define_operator!(
        ABS,
        "abs",
        [Matrix],
        {},
        1,
        Matrix,
        "Returns the absolute value of a number, removing any negative sign."
    );
    define_operator!(
        ADD,
        "add",
        [Matrix],
        { "filter" => (Boolean, Some(Constant::Boolean(false))) },
        -1,
        Matrix,
        "Add all inputs (at least 2 inputs required). If filter = true, filter all input NaN to 0 before adding"
    );
    define_operator!(
        ARC_TAN,
        "arc_tan",
        [Matrix],
        {},
        1,
        Matrix,
        "This operator does inverse tangent of input. Hence, constraints input within –pi/2 to pi/2 where ends. For very small values of x, arc_tan(x) = x. This is an odd and one-to-one function."
    );
    define_operator!(
        DENSIFY,
        "densify",
        [Group],
        {},
        1,
        Group,
        "Converts a grouping field of many buckets into lesser number of only available buckets so as to make working with grouping fields computationally efficient"
    );
    define_operator!(
        DIVIDE,
        "divide",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "x / y"
    );
    define_operator!(
        INVERSE,
        "inverse",
        [Matrix],
        {},
        1,
        Matrix,
        "1 / x"
    );
    define_operator!(
        LOG,
        "log",
        [Matrix],
        {},
        1,
        Matrix,
        "Calculates the natural logarithm of the input value. Commonly used to transform data that has positive values."
    );
    define_operator!(
        MAX,
        "max",
        [Matrix],
        {},
        -1,
        Matrix,
        "Maximum value of all inputs. At least 2 inputs are required"
    );
    define_operator!(
        MIN,
        "min",
        [Matrix],
        {},
        -1,
        Matrix,
        "Minimum value of all inputs. At least 2 inputs are required"
    );
    define_operator!(
        MULTIPLY,
        "multiply",
        [Matrix],
        { "filter" => (Boolean, Some(Constant::Boolean(false))) },
        -1,
        Matrix,
        "Multiplies two or more inputs element wise. Set filter=true to treat NaNs as 0 before multiplication"
    );
    define_operator!(
        NAN_MASK,
        "nan_mask",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "replace input with NAN if input's corresponding mask value or the second input here, is negative"
    );
    define_operator!(
        PASTEURIZE,
        "pasteurize",
        [Matrix],
        {},
        1,
        Matrix,
        "Set to NaN if x is INF or if the underlying instrument is not in the Alpha universe. This operator may help reduce outliers."
    );
    define_operator!(
        POWER,
        "power",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "x ^ y"
    );
    define_operator!(
        PURIFY,
        "purify",
        [Matrix],
        {},
        1,
        Matrix,
        "Clear infinities (+inf, -inf) by replacing with NaN"
    );
    define_operator!(
        REVERSE,
        "reverse",
        [Matrix],
        {},
        1,
        Matrix,
        "-x"
    );
    define_operator!(
        ROUND,
        "round",
        [Matrix],
        {},
        1,
        Matrix,
        "Round input to closest integer."
    );
    define_operator!(
        ROUND_DOWN,
        "round_down",
        [Matrix],
        {"f" => (Number, Some(Constant::Integer(1)))},
        1,
        Matrix,
        "Round input to greatest multiple of f less than input; Input: Value of 3 instruments at day t: (2.5, 3.2, 5.4), f: 1 Output: (2, 3, 5)"
    );
    define_operator!(
        SIGN,
        "sign",
        [Matrix],
        {},
        1,
        Matrix,
        "Returns the sign of a number: +1 for positive, -1 for negative, and 0 for zero. If the input is NaN, returns NaN."
    );
    define_operator!(
        SIGNED_POWER,
        "signed_power",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "x raised to the power of y such that final result preserves sign of x"
    );
    define_operator!(
        SQRT,
        "sqrt",
        [Matrix],
        {},
        1,
        Matrix,
        "Returns the non negative square root of x. Equivalent to power(x, 0.5); for signed roots use signed_power(x, 0.5)."
    );
    define_operator!(
        SUBTRACT,
        "subtract",
        [Matrix, Matrix],
        { "filter" => (Boolean, Some(Constant::Boolean(false))) },
        2,
        Matrix,
        "Subtracts inputs left to right. Supports only two inputs. Set filter=true to treat NaNs as 0 before subtraction."
    );
    define_operator!(
        TO_NAN,
        "to_nan",
        [Matrix],
        {
            "value" => (Number, Some(Constant::Integer(0))),
            "reverse" => (Boolean, Some(Constant::Boolean(false)))
        },
        1,
        Matrix,
        "Convert value to NaN or NaN to value if reverse=true"
    );
    define_operator!(
        AND,
        "and",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if both inputs are 1 ('true'). Otherwise, returns 0 ('false')."
    );
    define_operator!(
        IF_ELSE,
        "if_else",
        [Matrix, Matrix, Matrix],
        {},
        3,
        Matrix,
        "The if_else operator returns one of two values based on a condition. If the condition is true, it returns the first value; if false, it returns the second value."
    );
    define_operator!(
        LESS,
        "less",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 is a smaller than input2. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        LESS_EQUAL,
        "less_equal",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 is a smaller or the same as input2. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        EQUAL,
        "equal",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 and input2 are the same. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        GREATER,
        "greater",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 is a larger than input2. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        GREATER_EQUAL,
        "greater_equal",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 is a larger than input2 or equal to input2. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        NOT_EQUAL,
        "not_equal",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 ('true') if input1 and input2 are different numbers. Otherwise, returns 0 ('false')."
    );
    define_operator!(
        IS_NAN,
        "is_nan",
        [Matrix],
        {},
        1,
        Matrix,
        "If (input == NaN) return 1 else return 0"
    );
    define_operator!(
        IS_NOT_FINITE,
        "is_not_finite",
        [Matrix],
        {},
        1,
        Matrix,
        "If (input NAN or input == INF) return 1 else return 0"
    );
    define_operator!(
        NOT,
        "not",
        [Matrix],
        {},
        1,
        Matrix,
        "Returns the logical negation of x. Returns 0 when x is 1 ('true') and 1 when x is 0 ('false')."
    );
    define_operator!(
        OR,
        "or",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "Returns 1 if either input is true (either input1 or input2 has a value of 1), otherwise it returns 0."
    );
    define_operator!(
        DAYS_FROM_LAST_CHANGE,
        "days_from_last_change",
        [Matrix],
        {},
        1,
        Matrix,
        "Calculates the number of days since the last change in the value of a given variable."
    );
    define_operator!(
        HUMP,
        "hump",
        [Matrix],
        {"hump" => (PositiveFloat, Some(Constant::PositiveFloat(0.01)))},
        1,
        Matrix,
        "Limits amount and magnitude of changes in input (thus reducing turnover)"
    );
    define_operator!(
        JUMP_DECAY,
        "jump_decay",
        [Matrix, PositiveInt],
        {
            "stddev" => (Boolean, Some(Constant::Boolean(false))),
            "sensitivity" => (PositiveFloat, None),
            "force" => (PositiveFloat, None)
        },
        2,
        Matrix,
        "If there is a huge jump in current data compare to previous one, apply force: jump_decay(x) = abs(x-ts_delay(x, 1)) > sensitivity * ts_stddev(x,d) ? ts_delay(x,1) + ts_delta(x, 1) * force: x. If stddev enabled, jump threshold will be calculated as sensitivity * stddev otherwise it is sensitivity"
    );
    define_operator!(
        KTH_ELEMENT,
        "kth_element",
        [Matrix, PositiveInt],
        {
            "k" => (PositiveInt, None),
            "ignore" => (Set, Some(Constant::Set(vec![f64::NAN])))
        },
        2,
        Matrix,
        "Returns the K-th value from a time series by looking back over a specified number of ('d') days, with the option to ignore certain values. Commonly used for backfilling missing data."
    );
    define_operator!(
        LAST_DIFF_VALUE,
        "last_diff_value",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns the most recent value of x from the past d days that is different from the current value of x."
    );
    define_operator!(
        TS_ARG_MAX,
        "ts_arg_max",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns the number of days since the maximum value occurred in the last d days of a time series. If today's value is the maximum, returns 0; if it was yesterday, returns 1, and so on."
    );
    define_operator!(
        TS_ARG_MIN,
        "ts_arg_min",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns the number of days since the minimum value occurred in a time series over the past d days. If today's value is the minimum, returns 0; if it was yesterday, returns 1, and so on."
    );
    define_operator!(
        TS_AV_DIFF,
        "ts_av_diff",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Calculates the difference between a value and its mean over a specified period, ignoring NaN values in the mean calculation. In short, it returns x – ts_mean(x, d) with NaNs ignored."
    );
    define_operator!(
        TS_BACKFILL,
        "ts_backfill",
        [Matrix, PositiveInt],
        {"k" => (PositiveInt, Some(Constant::Integer(1)))},
        2,
        Matrix,
        "Replaces missing (NaN) values in a time series with the most recent valid value from a specified lookback window, improving data coverage and reducing risk from missing data."
    );
    define_operator!(
        TS_CO_SKEWNESS,
        "ts_co_skewness",
        [Matrix, Matrix, PositiveInt],
        {},
        3,
        Matrix,
        "Returns coskewness of y and x for the past d days"
    );
    define_operator!(
        TS_CORR,
        "ts_corr",
        [Matrix, Matrix, PositiveInt],
        {},
        3,
        Matrix,
        "Calculates the Pearson correlation between two variables, x and y, over the past d days, showing how closely they move together."
    );
    define_operator!(
        TS_COUNT_NANS,
        "ts_count_nans",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Counts the number of missing (NaN) values in a data series over a specified number of days."
    );
    define_operator!(
        TS_COVARIANCE,
        "ts_covariance",
        [Matrix, Matrix, PositiveInt],
        {},
        3,
        Matrix,
        "Calculates the covariance between two time-series variables, y and x, over the past d days. Useful for measuring how two variables move together within a specified historical window."
    );
    define_operator!(
        TS_DECAY_LINEAR,
        "ts_decay_linear",
        [Matrix, PositiveInt],
        {"dense" => (Boolean, Some(Constant::Boolean(false)))},
        2,
        Matrix,
        "Applies a linear decay to time-series data over a set number of days, smoothing the data by averaging recent values and reducing the impact of older or missing data."
    );
    define_operator!(
        TS_DELAY,
        "ts_delay",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns the value of a variable x from d days ago. Use this operator to access historical data points by specifying the desired time lag in days."
    );
    define_operator!(
        TS_DELTA,
        "ts_delta",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Calculates the difference between a value and its delayed version over a specified period. Useful for measuring changes or momentum in time-series data."
    );
    define_operator!(
        TS_IR,
        "ts_ir",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Return information ratio ts_mean(x, d) / ts_std_dev(x, d)"
    );
    define_operator!(
        TS_KURTOSIS,
        "ts_kurtosis",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns kurtosis of x for the last d days"
    );
    define_operator!(
        TS_MAX,
        "ts_max",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns max value of x for the past d days"
    );
    define_operator!(
        TS_MAX_DIFF,
        "ts_max_diff",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns x - ts_max(x, d)"
    );
    define_operator!(
        TS_MEAN,
        "ts_mean",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Calculates the simple average (mean) value of a variable x over the past d days."
    );
    define_operator!(
        TS_MIN,
        "ts_min",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns min value of x for the past d days"
    );
    define_operator!(
        TS_PRODUCT,
        "ts_product",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Returns the product of the values of x over the past d days. Useful for calculating geometric means and compounding returns or growth rates."
    );
    define_operator!(
        TS_QUANTILE,
        "ts_quantile",
        [Matrix, PositiveInt],
        {"driver" => (Driver, Some(Constant::Driver(Driver::Gaussian)))},
        2,
        Matrix,
        "Calculates the ts_rank of the input and transforms it using the inverse cumulative distribution function (quantile function) of a specified probability distribution (default: Gaussian/normal)."
    );
    define_operator!(
        TS_RANK,
        "ts_rank",
        [Matrix, PositiveInt],
        {"constant" => (Number, Some(Constant::Float(0.0)))},
        2,
        Matrix,
        "Ranks the value of a variable for each instrument over a specified number of past days, returning the rank of the current value (optionally adjusted by a constant)."
    );
    define_operator!(
        TS_RANK_GMEAN_AMEAN_DIFF,
        "ts_rank_gmean_amean_diff",
        [Matrix],
        {"lookback" => (PositiveInt, None)},
        -1,
        Matrix,
        "Returns Geometric Mean of ts_rank(input,d) of all input - Arithmetic Mean of ts_rank(input,d) of all input. This is similar to rank_gmean_amean_diff operator but in time-series space"
    );
    define_operator!(
        TS_REGRESSION,
        "ts_regression",
        [Matrix, Matrix, PositiveInt],
        {
            "lag" => (NonNegativeInt, Some(Constant::Zero)),
            "rettype" => (RetType, Some(Constant::Integer(0)))
        },
        3,
        Matrix,
        "Returns various parameters related to regression function, rettype can be 0~9: (0 Error Term), (1 y-intercept (α)), (2 slope (β)), (3 y-estimate), (4 Sum of Squares of Error (SSE)), (5 Sum of Squares of Total (SST)), (6 R-Square), (7 Mean Square Error (MSE)), (8 Standard Error of β), (9 Standard Error of α)"
    );
    define_operator!(
        TS_RETURNS,
        "ts_returns",
        [Matrix, PositiveInt],
        {"mode" => (ReturnMode, Some(Constant::Integer(1)))},
        2,
        Matrix,
        "Returns the relative change in the x value, mode can be 1 or 2, If mode = 1, it returns (x – ts_delay(x, d )) / ts_delay(x, d), If mode = 2, it returns mode == 2: (x – ts_delay(x, d )) / ((x + ts_delay(x, d))/2)"
    );
    define_operator!(
        TS_SCALE,
        "ts_scale",
        [Matrix, PositiveInt],
        {"constant" => (Number, Some(Constant::Float(0.0)))},
        2,
        Matrix,
        "Scales a time series to a 0–1 range based on its minimum and maximum values over a specified period, with an optional constant shift."
    );
    define_operator!(
        TS_STD_DEV,
        "ts_std_dev",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Calculates the standard deviation of a data series x over the past d days, measuring how much the values deviate from their mean during that period."
    );
    define_operator!(
        TS_STEP,
        "ts_step",
        [Int],
        {},
        1,
        Matrix,
        "Returns a counter of days, incrementing by one each day."
    );
    define_operator!(
        TS_SUM,
        "ts_sum",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Sum values of x for the past d days."
    );
    define_operator!(
        TS_TARGET_TVR_DECAY,
        "ts_target_tvr_decay",
        [Matrix],
        {
            "lambda_min" => (NonNegativeFloat, Some(Constant::Zero)),
            "lambda_max" => (NonNegativeFloat, Some(Constant::One)),
            "target_tvr" => (Ratio, None)
        },
        1,
        Matrix,
        "Tune ts_decay to have a turnover equal to a certain target, with optimization weight range between lambda_min, lambda_max"
    );
    define_operator!(
        TS_TARGET_TVR_DELTA_LIMIT,
        "ts_target_tvr_delta_limit",
        [Matrix, Matrix],
        {
            "lambda_min" => (NonNegativeFloat, Some(Constant::Zero)),
            "lambda_max" => (NonNegativeFloat, Some(Constant::One)),
            "target_tvr" => (Ratio, None)
        },
        2,
        Matrix,
        "Tune ts_delta_limit to have a turnover equal to a certain target with optimization weight range between lambda_min, lambda_max. Also, please be aware of the scaling for x and y. Besides setting y as adv20 or volume related data, you can also set y as a constant."
    );
    define_operator!(
        TS_TARGET_TVR_HUMP,
        "ts_target_tvr_hump",
        [Matrix],
        {
            "lambda_min" => (NonNegativeFloat, Some(Constant::Zero)),
            "lambda_max" => (NonNegativeFloat, Some(Constant::One)),
            "target_tvr" => (Ratio, None)
        },
        1,
        Matrix,
        "Tune hump to have a turnover equal to a certain target with optimization weight range between lambda_min, lambda_max."
    );
    define_operator!(
        TS_TRIPLE_CORR,
        "ts_triple_corr",
        [Matrix, Matrix, Matrix, PositiveInt],
        {},
        4,
        Matrix,
        "Returns triple correlation of x, y, z for the past d days"
    );
    define_operator!(
        TS_VECTOR_PROJ,
        "ts_vector_proj",
        [Matrix, Matrix, PositiveInt],
        {},
        3,
        Matrix,
        "Returns vector projection of x onto y in time-series space."
    );
    define_operator!(
        TS_VECTOR_NEUT,
        "ts_vector_neut",
        [Matrix, Matrix, PositiveInt],
        {},
        3,
        Matrix,
        "Returns x- ts_vector_proj(x,y,d)"
    );
    define_operator!(
        TS_ZSCORE,
        "ts_zscore",
        [Matrix, PositiveInt],
        {},
        2,
        Matrix,
        "Calculates the Z-score of a time series, showing how far today's value is from the recent average, measured in standard deviations."
    );
    define_operator!(
        NORMALIZE,
        "normalize",
        [Matrix],
        {
            "useStd" => (Boolean, Some(Constant::Boolean(false))),
            "limit" => (Number, Some(Constant::Float(0.0)))
        },
        1,
        Matrix,
        "Calculates the mean value of all valid alpha values for a certain date, then subtracts that mean from each element"
    );
    define_operator!(
        QUANTILE,
        "quantile",
        [Matrix],
        {
            "driver" => (Driver, Some(Constant::Driver(Driver::Gaussian))),
            "sigma" => (PositiveFloat, Some(Constant::PositiveFloat(1.0)))
        },
        1,
        Matrix,
        "Rank the raw vector, shift the ranked Alpha vector, apply distribution (gaussian, cauchy, uniform). If driver is uniform, it simply subtract each Alpha value with the mean of all Alpha values in the Alpha vector"
    );
    define_operator!(
        RANK,
        "rank",
        [Matrix],
        {
            "rate" => (RankRate, Some(Constant::Integer(2)))
        },
        1,
        Matrix,
        "Ranks the input among all the instruments and returns an equally distributed number between 0.0 and 1.0. For precise sort, use the rate as 0"
    );
    define_operator!(
        SCALE,
        "scale",
        [Matrix],
        {
            "scale" => (PositiveFloat, Some(Constant::Integer(1))),
            "longscale" => (PositiveFloat, Some(Constant::Integer(1))),
            "shortscale" => (PositiveFloat, Some(Constant::Integer(1)))
        },
        1,
        Matrix,
        "Scales input to booksize. We can also scale the long positions and short positions to separate scales by mentioning additional parameters to the operator, The scale(x, scale=1, longscale=1, shortscale=1) operator adjusts the input values so that their total absolute value matches a target book size. By default, it scales so that the sum of absolute values is 1, but you can set a different scale. You can also use longscale and shortscale to apply different scaling to long and short positions, respectively. This operator is useful for normalizing your alpha signals and reducing the impact of outliers."
    );
    define_operator!(
        SCALE_DOWN,
        "scale_down",
        [Matrix],
        {"constant" => (Number, Some(Constant::Integer(0)))},
        1,
        Matrix,
        "Scales all values in each day proportionately between 0 and 1 such that minimum value maps to 0 and maximum value maps to 1. Constant is the offset by which final result is subtracted"
    );
    define_operator!(
        VECTOR_NEUT,
        "vector_neut",
        [Matrix, Matrix],
        {},
        2,
        Matrix,
        "For given vectors x and y, it finds a new vector x* (output) such that x* is orthogonal to y"
    );
    define_operator!(
        WINSORIZE,
        "winsorize",
        [Matrix],
        {
            "std" => (PositiveFloat, Some(Constant::PositiveFloat(4.0)))
        },
        1,
        Matrix,
        "Winsorizes x to make sure that all values in x are between the lower and upper limits, which are specified as multiple of std."
    );
    define_operator!(
        ZSCORE,
        "zscore",
        [Matrix],
        {},
        1,
        Matrix,
        "Z-score is a numerical measurement that describes a value's relationship to the mean of a group of values. Z-score is measured in terms of standard deviations from the mean"
    );
    define_operator!(
        VEC_AVG,
        "vec_avg",
        [Vector],
        {},
        1,
        Matrix,
        "Taking mean of the vector field x"
    );
    define_operator!(
        VEC_COUNT,
        "vec_count",
        [Vector],
        {},
        1,
        Matrix,
        "Number of elements in vector field x"
    );
    define_operator!(
        VEC_MAX,
        "vec_max",
        [Vector],
        {},
        1,
        Matrix,
        "Maximum value from vector field x"
    );
    define_operator!(
        VEC_MIN,
        "vec_min",
        [Vector],
        {},
        1,
        Matrix,
        "Minimum value from vector field x"
    );
    define_operator!(
        VEC_NORM,
        "vec_norm",
        [Vector],
        {},
        1,
        Matrix,
        "Sum of all absolute values of vector field x"
    );
    define_operator!(
        VEC_RANGE,
        "vec_range",
        [Vector],
        {},
        1,
        Matrix,
        "Difference between maximum and minimum element in vector field x"
    );
    define_operator!(
        VEC_STDDEV,
        "vec_stddev",
        [Vector],
        {},
        1,
        Matrix,
        "Standard Deviation of vector field x"
    );
    define_operator!(
        VEC_SUM,
        "vec_sum",
        [Vector],
        {},
        1,
        Matrix,
        "Sum of vector field x"
    );
    define_operator!(
        BUCKET,
        "bucket",
        [Matrix],
        {
            "range" => (Range, Some(Constant::NaN)),
            "buckets" => (Array, Some(Constant::NaN)),
            "skipBoth" => (Boolean, Some(Constant::Boolean(false))),
            "NaNGroup" => (Boolean, Some(Constant::Boolean(false))),
        },
        1,
        Group,
        "Convert float values into indexes for user-specified buckets. Bucket is useful for creating group values, which can be passed to GROUP as input.\nrank(x) transforms the input into a uniform distribution between 0 and 1.\nbucket(...) splits these ranked values into discrete groups (buckets) based on your chosen method:\n1. Range: range=“start, end, step” divides the interval [start, end] into equal-width buckets.\n2. Buckets: buckets=“num_1,num_2,...,num_N” creates buckets with custom boundaries.\nTwo hidden buckets corresponding to (-inf, start] and [end, +inf) are added by default are added by default. The optional parameter “skipBoth”, “skipBegin” and “skipEnd” can be set to “True” to remove these buckets and give NAN for the values that are out of range.\nBy setting NANGroup = True, all NAN input values will be in the new bucket which will be index as the last bucket.\nuse range or buckets, **DO NOT** use both.\nmost common usage is bucket(rank(x), range='0,1,0.1')"
    );
    define_operator!(
        CLAMP,
        "clamp",
        [Matrix],
        {
            "lower" => (Number, None),
            "upper" => (Number, None),
            "inverse" => (Boolean, Some(Constant::Boolean(false))),
            "mask" => (Mask, Some(Constant::NaN)),
        },
        1,
        Matrix,
        "Limits input value between lower and upper bound in inverse = false mode (which is default). Alternatively, when inverse = true, values between bounds are replaced with mask, while values outside bounds are left as is"
    );
    define_operator!(
        FILTER,
        "filter",
        [Matrix],
        {
            "h" => (Array, None),
            "t" => (Array, None),
        },
        1,
        Matrix,
        "Scales all values in each day proportionately between 0 and 1 such that minimum value maps to 0 and maximum value maps to 1. Constant is the offset by which final result is subtracted. This operator is used to filter the value. It is calculated as follows: \noutput[t] = h[0] * input[t-1] + h[1] * input[t-2] + ... + t[0] * output[t-1] + t[1] * output[t-2] + ... \nWhere h and t are vectors of floats separated by comma. \nThus it can be divided into two parts: one that acts like weighted moving average (h) and recursive part (t). This allows to create different kinds of filters like decay linear or exponential decay."
    );
    define_operator!(
        KEEP,
        "keep",
        [Matrix, Matrix],
        {"period" => (PositiveInt, Some(Constant::Integer(5)))},
        2,
        Matrix,
        "This operator outputs value x when f changes and continues to do that for \"period\" days after f stopped changing. After \"period\" days since last change of f, NaN is output.\nThis can be expressed as the below code:\nD = days_from_last_change(f); u = trade_when(D < period, x, D > period); u"
    );
    define_operator!(
        TAIL,
        "tail",
        [Matrix],
        {
            "lower" => (Number, Some(Constant::Integer(0))),
            "upper" => (Number, Some(Constant::Integer(0))),
            "newval" => (Number, Some(Constant::Integer(0)))
        },
        1,
        Matrix,
        "If (x > lower AND x < upper) return newval, else return x. Lower, upper, newval should be constants"
    );
    define_operator!(
        TRADE_WHEN,
        "trade_when",
        [Matrix, Matrix, Matrix],
        {},
        3,
        Matrix,
        "Used in order to change Alpha values only under a specified condition and to hold Alpha values in other cases. It also allows to close Alpha positions (assign NaN values) under a specified condition"
    );
    define_operator!(
        GROUP_BACKFILL,
        "group_backfill",
        [Matrix, Group, PositiveInt],
        {"std" => (PositiveFloat, Some(Constant::Float(4.0)))},
        3,
        Matrix,
        "If a certain value for a certain date and instrument is NaN, from the set of same group instruments, calculate winsorized mean of all non-NaN values over last d days"
    );
    define_operator!(
        GROUP_CARTESIAN_PRODUCT,
        "group_cartesian_product",
        [Group, Group],
        {},
        2,
        Group,
        "Merge two groups into one group. If originally there are len_1 and len_2 group indices in g1 and g2, there will be len_1 * len_2 indices in the new group."
    );
    define_operator!(
        GROUP_COUNT,
        "group_count",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Gives the number of instruments in the same group (e.g. sector) which have valid values of x. This operator improves weight coverage and may help to reduce drawdown risk."
    );
    define_operator!(
        GROUP_MAX,
        "group_max",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Maximum of x for all instruments in the same group."
    );
    define_operator!(
        GROUP_MEAN,
        "group_mean",
        [Matrix, Matrix, Group],
        {},
        3,
        Matrix,
        "All elements in group equals to the mean"
    );
    define_operator!(
        GROUP_MIN,
        "group_min",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "All elements in group equals to the min value of the group."
    );
    define_operator!(
        GROUP_NEUTRALIZE,
        "group_neutralize",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Neutralizes Alpha against groups. These groups can be subindustry, industry, sector, country or a constant"
    );
    define_operator!(
        GROUP_RANK,
        "group_rank",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Each elements in a group is assigned the corresponding rank in this group"
    );
    define_operator!(
        GROUP_SCALE,
        "group_scale",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Normalizes the values in a group to be between 0 and 1. (x - groupmin) / (groupmax - groupmin)"
    );
    define_operator!(
        GROUP_STD_DEV,
        "group_std_dev",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "All elements in group equals to the standard deviation of the group."
    );
    define_operator!(
        GROUP_SUM,
        "group_sum",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Sum of x for all instruments in the same group."
    );
    define_operator!(
        GROUP_VECTOR_NEUT,
        "group_vector_neut",
        [Matrix, Matrix, Group],
        {},
        3,
        Matrix,
        "Similar to vector_neut(x, y) but x neutralize to y for each group g which can be any classifier such as subindustry, industry, sector, etc."
    );
    define_operator!(
        GROUP_ZSCORE,
        "group_zscore",
        [Matrix, Group],
        {},
        2,
        Matrix,
        "Calculates group Z-score - numerical measurement that describes a value's relationship to the mean of a group of values. Z-score is measured in terms of standard deviations from the mean."
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Constant, Field, OPERATORS, OperatorCheckError, get_operator};

    fn matrices(count: usize) -> Vec<Field> {
        vec![Field::Matrix; count]
    }

    #[test]
    fn registry_contains_unique_named_operators() {
        assert!(OPERATORS.len() >= 100);
        for (index, left) in OPERATORS.iter().enumerate() {
            for right in OPERATORS.iter().skip(index + 1) {
                assert_ne!(left.name, right.name);
            }
            assert_eq!(
                get_operator(left.name).map(|operator| operator.name),
                Some(left.name)
            );
        }
    }

    #[test]
    fn applies_operator_with_valid_positional_and_default_keyword_args() {
        let add = get_operator("add").expect("add must be registered");
        let result = add.apply(&matrices(2), &HashMap::new());
        assert!(matches!(result, Ok(Field::Matrix)));
    }

    #[test]
    fn rejects_invalid_positional_argument_counts() {
        let add = get_operator("add").unwrap();
        assert!(matches!(
            add.apply(&matrices(1), &HashMap::new()),
            Err(OperatorCheckError::NaryPosArgsLessThanTwo)
        ));

        let divide = get_operator("divide").unwrap();
        assert!(matches!(
            divide.apply(&matrices(1), &HashMap::new()),
            Err(OperatorCheckError::InvalidPositionalArgumentCount {
                expected: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn validates_keyword_names_types_and_required_arguments() {
        let add = get_operator("add").unwrap();

        let mut valid = HashMap::new();
        valid.insert("filter".to_string(), Constant::Boolean(true));
        assert!(matches!(add.apply(&matrices(2), &valid), Ok(Field::Matrix)));

        let mut wrong_type = HashMap::new();
        wrong_type.insert("filter".to_string(), Constant::Integer(1));
        assert!(matches!(
            add.apply(&matrices(2), &wrong_type),
            Err(OperatorCheckError::InvalidKeywordType { name, .. }) if name == "filter"
        ));

        let mut unknown = HashMap::new();
        unknown.insert("missing".to_string(), Constant::Boolean(false));
        assert!(matches!(
            add.apply(&matrices(2), &unknown),
            Err(OperatorCheckError::UnknownKeyword(name)) if name == "missing"
        ));

        let required = get_operator("ts_rank_gmean_amean_diff").unwrap();
        assert!(matches!(
            required.apply(&matrices(2), &HashMap::new()),
            Err(OperatorCheckError::MissingKeyword(name)) if name == "lookback"
        ));

        let kth = get_operator("kth_element").unwrap();
        assert!(matches!(
            kth.apply(
                &[
                    Field::Matrix,
                    Field::Constant(Constant::PositiveInteger(5)),
                ],
                &HashMap::new(),
            ),
            Err(OperatorCheckError::MissingKeyword(name)) if name == "k"
        ));
    }

    #[test]
    fn validates_operator_specific_constraints() {
        let kth = get_operator("kth_element").unwrap();
        let args = vec![Field::Matrix, Field::Constant(Constant::PositiveInteger(3))];
        let mut kth_kwargs = HashMap::new();
        kth_kwargs.insert("k".to_string(), Constant::PositiveInteger(4));
        assert!(matches!(
            kth.apply(&args, &kth_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("k <= d")
        ));

        let backfill = get_operator("ts_backfill").unwrap();
        let mut backfill_kwargs = HashMap::new();
        backfill_kwargs.insert("k".to_string(), Constant::PositiveInteger(6));
        assert!(matches!(
            backfill.apply(
                &[
                    Field::Matrix,
                    Field::Constant(Constant::PositiveInteger(5)),
                ],
                &backfill_kwargs,
            ),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("k <= d")
        ));

        let bucket = get_operator("bucket").unwrap();
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &HashMap::new()),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("exactly one")
        ));

        let mut bucket_kwargs = HashMap::new();
        bucket_kwargs.insert("range".to_string(), Constant::Range(0.1));
        bucket_kwargs.insert("buckets".to_string(), Constant::Array(vec![0.0, 1.0]));
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("exactly one")
        ));

        bucket_kwargs.insert("range".to_string(), Constant::Range(0.1));
        bucket_kwargs.insert("buckets".to_string(), Constant::Range(0.1));
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("Array, not a Range")
        ));

        bucket_kwargs.insert("range".to_string(), Constant::Range(0.1));
        bucket_kwargs.insert("buckets".to_string(), Constant::Array(vec![0.0, 0.0, 1.0]));
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("strictly increasing")
        ));

        bucket_kwargs.insert("buckets".to_string(), Constant::Array(vec![1.0, 0.0]));
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("strictly increasing")
        ));

        bucket_kwargs.insert("range".to_string(), Constant::NaN);
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidKeywordType { name, .. })
                if name == "range"
        ));

        bucket_kwargs.insert("range".to_string(), Constant::Range(0.1));
        bucket_kwargs.insert("buckets".to_string(), Constant::NaN);
        assert!(matches!(
            bucket.apply(&[Field::Matrix], &bucket_kwargs),
            Err(OperatorCheckError::InvalidKeywordType { name, .. })
                if name == "buckets"
        ));

        let clamp = get_operator("clamp").unwrap();
        let mut clamp_kwargs = HashMap::new();
        clamp_kwargs.insert("lower".to_string(), Constant::Integer(2));
        clamp_kwargs.insert("upper".to_string(), Constant::Integer(1));
        clamp_kwargs.insert("inverse".to_string(), Constant::Boolean(false));
        assert!(matches!(
            clamp.apply(&[Field::Matrix], &clamp_kwargs),
            Err(OperatorCheckError::InvalidExpression(message))
                if message.contains("upper > lower")
        ));
    }
}
