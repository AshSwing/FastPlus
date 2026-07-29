//! AST
//! 唯一结果对象: Alpha
//!     |-> [Assignment;]         赋值语句
//!     |-> [Assignment;]         赋值语句
//!     |-> [...;]                ...
//!     |-> signal(Expression)    信号表达式
//! Alpha 由 N 个赋值语句和 1 个信号表达式组成
//!
//! Assignment 赋值语句
//!     |-> variable(String) = value(Expression);
//!
//! 核心结构: Expression 表达式
//!     |-> Group(Expression) / Ternary / Binary / Unary 递归结构
//!     |-> Variable / Placeholder                       占位结构
//!     |-> Operation*                                   操作符*
//!     |-> Constant*                                    常数项*
//!
//! 特殊规则:
//!   - 不可以给常数赋值 ❌    -> 刚试了下又可以了(2026-07-28)
//!   - 不可复用数据字段名     -> 非左值 Variable 都是数据字段
//!   - 参数支持字符串/字面值  -> 自动转为字符串解析
//!   - 支持 NaN, 不支持 Inf

use std::collections::HashMap;

use crate::field::Constant;

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
