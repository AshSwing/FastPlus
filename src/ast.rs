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
use std::collections::HashSet;

use crate::field::Constant;
use crate::field::Field;
use crate::operator::get_operator;

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "fastplus.pest"]
pub struct FastPlusParser;

#[derive(Debug, Clone)]
pub struct Alpha {
    assignments: Vec<Assignment>,
    signal: Expression,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlphaFields {
    pub matrix: Vec<String>,
    pub vector: Vec<String>,
    pub group: Vec<String>,
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

    pub fn fields(&self) -> AlphaFields {
        let defined: HashSet<_> = self
            .assignments
            .iter()
            .map(|assignment| assignment.variable.clone())
            .collect();
        let mut fields = AlphaFields::default();

        for assignment in &self.assignments {
            collect_fields(
                &assignment.value,
                &Field::Matrix,
                &defined,
                &mut fields,
                &mut Vec::new(),
            );
        }
        collect_fields(
            &self.signal,
            &Field::Matrix,
            &defined,
            &mut fields,
            &mut Vec::new(),
        );
        fields
    }

    pub fn operators(&self) -> Vec<String> {
        let mut operators = Vec::new();
        for assignment in &self.assignments {
            collect_operators(&assignment.value, &mut operators);
        }
        collect_operators(&self.signal, &mut operators);
        operators
    }
}

fn collect_fields(
    expression: &Expression,
    expected: &Field,
    defined: &HashSet<String>,
    fields: &mut AlphaFields,
    operators: &mut Vec<String>,
) {
    match expression {
        Expression::Variable(name) if !defined.contains(name) => {
            add_field(fields, expected, name.clone());
        }
        Expression::Variable(_) => {}
        Expression::Placeholder(name) => add_field(fields, expected, name.clone()),
        Expression::Group(expression) => {
            collect_fields(expression, expected, defined, fields, operators)
        }
        Expression::Operation {
            op,
            pos_args,
            kw_args: _,
        } => {
            add_operator(operators, op);
            if let Some(operator) = get_operator(op) {
                for (index, argument) in pos_args.iter().enumerate() {
                    if let Some(argument_type) = if operator.nary == -1 {
                        operator.pos_args.first()
                    } else {
                        operator.pos_args.get(index)
                    } {
                        collect_fields(argument, argument_type, defined, fields, operators);
                    } else {
                        collect_fields(argument, &Field::Matrix, defined, fields, operators);
                    }
                }
            } else {
                for argument in pos_args {
                    collect_fields(argument, &Field::Matrix, defined, fields, operators);
                }
            }
        }
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_fields(cond, &Field::Matrix, defined, fields, operators);
            collect_fields(then_branch, &Field::Matrix, defined, fields, operators);
            collect_fields(else_branch, &Field::Matrix, defined, fields, operators);
        }
        Expression::Binary { left, right, .. } => {
            collect_fields(left, &Field::Matrix, defined, fields, operators);
            collect_fields(right, &Field::Matrix, defined, fields, operators);
        }
        Expression::Unary { expr, .. } => {
            collect_fields(expr, &Field::Matrix, defined, fields, operators)
        }
        Expression::Constant(_) => {}
    }
}

fn collect_operators(expression: &Expression, operators: &mut Vec<String>) {
    match expression {
        Expression::Operation { op, pos_args, .. } => {
            add_operator(operators, op);
            for argument in pos_args {
                collect_operators(argument, operators);
            }
        }
        Expression::Group(expression) => collect_operators(expression, operators),
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_operators(cond, operators);
            collect_operators(then_branch, operators);
            collect_operators(else_branch, operators);
        }
        Expression::Binary { left, right, .. } => {
            collect_operators(left, operators);
            collect_operators(right, operators);
        }
        Expression::Unary { expr, .. } => collect_operators(expr, operators),
        Expression::Constant(_) | Expression::Placeholder(_) | Expression::Variable(_) => {}
    }
}

fn add_field(fields: &mut AlphaFields, expected: &Field, name: String) {
    let target = match expected {
        Field::Vector => &mut fields.vector,
        Field::Group => &mut fields.group,
        _ => &mut fields.matrix,
    };
    if !target.contains(&name) {
        target.push(name);
    }
}

fn add_operator(operators: &mut Vec<String>, name: &str) {
    if !operators.iter().any(|operator| operator == name) {
        operators.push(name.to_string());
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
