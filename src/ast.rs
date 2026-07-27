use std::collections::HashMap;

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "fastplus.pest"]
pub struct FastPlusParser;

#[derive(Debug, Clone)]
pub struct Alpha {
    assignments: Vec<Assignment>,
    signal: Expression,
}

impl Alpha {
    pub fn new(assignments: Vec<Assignment>, signal: Expression) -> Self {
        Alpha {
            assignments,
            signal,
        }
    }

    pub fn signal(&self) -> &Expression {
        &self.signal
    }

    pub fn assignments(&self) -> &[Assignment] {
        &self.assignments
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    variable: String,
    value: Expression,
}

impl Assignment {
    pub fn new(variable: String, value: Expression) -> Self {
        Assignment { variable, value }
    }

    pub fn variable(&self) -> &str {
        &self.variable
    }

    pub fn value(&self) -> &Expression {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    NotEqual,     // !=
    Equal,        // ==
    GreaterEqual, // >=
    LessEqual,    // <=
    Greater,      // >
    Less,         // <
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Or,           // ||
    And,          // &&
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Positive, // +
    Negative, // -
    Not,      // !
}

#[derive(Debug, Clone)]
pub enum Driver {
    Gaussian,
    Uniform,
    Cauchy,
}

#[derive(Debug, Clone)]
pub enum Constant {
    Float(f64),
    NonNegativeFloat(f64),
    PositiveFloat(f64),
    Ratio(f64),
    Integer(i64),
    NonNegativeInteger(u64),
    PositiveInteger(u64),
    EnumInteger { from: i64, to: i64 },
    Boolean(bool),   // true / false
    NaN,             // "NaN"
    Driver(Driver),  // "gaussian"
    Range(f64),      // 0, 1, 0.1
    Array(Vec<f64>), // buckets, filter(h, t)
    String(String),
}

#[derive(Debug, Clone)]
pub enum Expression {
    Ternary {
        // cond ? true_val : false_val
        cond: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    Binary {
        // left op right
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        // op expr
        op: UnaryOp,
        expr: Box<Expression>,
    },
    Operation {
        // op(args)
        op: String,
        pos_args: Vec<Expression>,
        kw_args: HashMap<String, Constant>,
    },
    Constant(Constant),
    Placeholder(String),
    Variable(String),
    Group(Box<Expression>),
}
