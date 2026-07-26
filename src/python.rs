#[macro_use]
#[path = "python_macro.rs"]
mod python_macro;

use crate::ast::{Alpha, Assignment, BinaryOp, Constant, Driver, Expression, KwArg, UnaryOp};
use crate::parse as parse_rust;
use pyo3::prelude::*;

py_class!(
    (PyAlpha(Alpha), name = "Alpha"),
    (PyAssignment(Assignment), name = "Assignment"),
    (PyKwArg(KwArg), name = "KwArg"),
    (PyExpression, name = "Expression", attrs = [subclass]),
    (
        PyTernaryExpr {
            cond: Expression,
            then_branch: Expression,
            else_branch: Expression,
        },
        name = "TernaryExpr",
        super = PyExpression
    ),
    (
        PyBinaryExpr {
            left: Expression,
            op: PyBinaryOp,
            right: Expression,
        },
        name = "BinaryExpr",
        super = PyExpression
    ),
    (
        PyUnaryExpr {
            op: PyUnaryOp,
            expr: Expression,
        },
        name = "UnaryExpr",
        super = PyExpression
    ),
    (
        PyOperationExpr {
            op: String,
            pos_args: Vec<Expression>,
            kw_args: Vec<KwArg>,
        },
        name = "OperationExpr",
        super = PyExpression
    ),
    (
        PyConstant,
        name = "Constant",
        super = PyExpression,
        attrs = [subclass]
    ),
    (PyFloatConstant(f64), name = "FloatConstant", super = PyConstant),
    (
        PyNonNegativeFloatConstant(f64),
        name = "NonNegativeFloatConstant",
        super = PyConstant
    ),
    (
        PyPositiveFloatConstant(f64),
        name = "PositiveFloatConstant",
        super = PyConstant
    ),
    (PyRatioConstant(f64), name = "RatioConstant", super = PyConstant),
    (PyIntegerConstant(i64), name = "IntegerConstant", super = PyConstant),
    (
        PyNonNegativeIntegerConstant(u64),
        name = "NonNegativeIntegerConstant",
        super = PyConstant
    ),
    (
        PyPositiveIntegerConstant(u64),
        name = "PositiveIntegerConstant",
        super = PyConstant
    ),
    (
        PyEnumIntegerConstant {
            value_from: i64,
            value_to: i64,
        },
        name = "EnumIntegerConstant",
        super = PyConstant
    ),
    (PyBooleanConstant(bool), name = "BooleanConstant", super = PyConstant),
    (PyNaNConstant, name = "NaNConstant", super = PyConstant),
    (PyDriverConstant(Driver), name = "DriverConstant", super = PyConstant),
    (PyRangeConstant(f64), name = "RangeConstant", super = PyConstant),
    (PyArrayConstant(Vec<f64>), name = "ArrayConstant", super = PyConstant),
    (PyStringConstant(String), name = "StringConstant", super = PyConstant),
    (
        PyPlaceholderExpr(String),
        name = "PlaceholderExpr",
        super = PyExpression
    ),
    (PyVariableExpr(String), name = "VariableExpr", super = PyExpression),
    (
        PyGroupExpr { expr: Expression },
        name = "GroupExpr",
        super = PyExpression
    ),
);

py_enum!(
    (
        PyBinaryOp,
        BinaryOp,
        "BinaryOp",
        [
            NotEqual,
            Equal,
            GreaterEqual,
            LessEqual,
            Greater,
            Less,
            Add,
            Subtract,
            Multiply,
            Divide,
            Or,
            And,
        ]
    ),
    (PyUnaryOp, UnaryOp, "UnaryOp", [Positive, Negative, Not]),
    (PyDriver, Driver, "Driver", [Gaussian, Uniform, Cauchy]),
);

fn build_pyconstant(py: Python<'_>, value: Constant) -> PyResult<Py<PyExpression>> {
    Ok(match value {
        Constant::Float(value) => build_pyconstant_instance!(py, PyFloatConstant(value)),
        Constant::NonNegativeFloat(value) => {
            build_pyconstant_instance!(py, PyNonNegativeFloatConstant(value))
        }
        Constant::PositiveFloat(value) => {
            build_pyconstant_instance!(py, PyPositiveFloatConstant(value))
        }
        Constant::Ratio(value) => build_pyconstant_instance!(py, PyRatioConstant(value)),
        Constant::Integer(value) => build_pyconstant_instance!(py, PyIntegerConstant(value)),
        Constant::NonNegativeInteger(value) => {
            build_pyconstant_instance!(py, PyNonNegativeIntegerConstant(value))
        }
        Constant::PositiveInteger(value) => {
            build_pyconstant_instance!(py, PyPositiveIntegerConstant(value))
        }
        Constant::EnumInteger { from, to } => {
            build_pyconstant_instance!(
                py,
                PyEnumIntegerConstant {
                    value_from: from,
                    value_to: to
                }
            )
        }
        Constant::Boolean(value) => build_pyconstant_instance!(py, PyBooleanConstant(value)),
        Constant::NaN => build_pyconstant_instance!(py, PyNaNConstant),
        Constant::Driver(value) => build_pyconstant_instance!(py, PyDriverConstant(value)),
        Constant::Range(value) => build_pyconstant_instance!(py, PyRangeConstant(value)),
        Constant::Array(value) => build_pyconstant_instance!(py, PyArrayConstant(value)),
        Constant::String(value) => build_pyconstant_instance!(py, PyStringConstant(value)),
    })
}

fn build_pyexpression(py: Python<'_>, expression: Expression) -> PyResult<Py<PyExpression>> {
    Ok(match expression {
        Expression::Ternary {
            cond,
            then_branch,
            else_branch,
        } => build_pyexpression_instance!(
            py,
            PyTernaryExpr {
                cond: *cond,
                then_branch: *then_branch,
                else_branch: *else_branch,
            }
        ),
        Expression::Binary { left, op, right } => build_pyexpression_instance!(
            py,
            PyBinaryExpr {
                left: *left,
                op: op.into(),
                right: *right,
            }
        ),
        Expression::Unary { op, expr } => build_pyexpression_instance!(
            py,
            PyUnaryExpr {
                op: op.into(),
                expr: *expr,
            }
        ),
        Expression::Operation {
            op,
            pos_args,
            kw_args,
        } => build_pyexpression_instance!(
            py,
            PyOperationExpr {
                op,
                pos_args,
                kw_args,
            }
        ),
        Expression::Constant(value) => return build_pyconstant(py, value),
        Expression::Placeholder(name) => {
            build_pyexpression_instance!(py, PyPlaceholderExpr(name))
        }
        Expression::Variable(name) => build_pyexpression_instance!(py, PyVariableExpr(name)),
        Expression::Group(expr) => {
            build_pyexpression_instance!(py, PyGroupExpr { expr: *expr })
        }
    })
}

py_getters!(
    (
        PyBinaryExpr,
        {
            left: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.left.clone())
            },
            right: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.right.clone())
            },
            op: PyBinaryOp, |self, _py| { self.op }
        }
    ),
    (
        PyUnaryExpr,
        {
            expr: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.expr.clone())
            },
            op: PyUnaryOp, |self, _py| { self.op }
        }
    ),
    (
        PyTernaryExpr,
        {
            cond: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.cond.clone())
            },
            then_branch: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.then_branch.clone())
            },
            else_branch: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.else_branch.clone())
            }
        }
    ),
    (
        PyGroupExpr,
        {
            expr: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.expr.clone())
            }
        }
    ),
    (
        PyAlpha,
        {
            assignments: Vec<PyAssignment>, |self, _py| {
                self.0
                    .assignments()
                    .iter()
                    .cloned()
                    .map(PyAssignment)
                    .collect()
            },
            signal: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.0.signal().clone())
            }
        }
    ),
    (
        PyAssignment,
        {
            variable: &str, |self, _py| { self.0.variable() },
            value: PyResult<Py<PyExpression>>, |self, py| {
                build_pyexpression(py, self.0.value().clone())
            }
        }
    ),
    (
        PyKwArg,
        {
            name: &str, |self, _py| { &self.0.name },
            value: PyResult<Py<PyExpression>>, |self, py| {
                build_pyconstant(py, self.0.value.clone())
            }
        }
    ),
    (
        PyOperationExpr,
        {
            op: &str, |self, _py| { &self.op },
            pos_args: PyResult<Vec<Py<PyExpression>>>, |self, py| {
                self.pos_args
                    .iter()
                    .cloned()
                    .map(|expr| build_pyexpression(py, expr))
                    .collect()
            },
            kw_args: Vec<PyKwArg>, |self, _py| {
                self.kw_args.iter().cloned().map(PyKwArg).collect()
            }
        }
    ),
    (
        PyFloatConstant,
        { value: f64, |self, _py| { self.0 } }
    ),
    (
        PyNonNegativeFloatConstant,
        { value: f64, |self, _py| { self.0 } }
    ),
    (
        PyPositiveFloatConstant,
        { value: f64, |self, _py| { self.0 } }
    ),
    (
        PyRatioConstant,
        { value: f64, |self, _py| { self.0 } }
    ),
    (
        PyIntegerConstant,
        { value: i64, |self, _py| { self.0 } }
    ),
    (
        PyNonNegativeIntegerConstant,
        { value: u64, |self, _py| { self.0 } }
    ),
    (
        PyPositiveIntegerConstant,
        { value: u64, |self, _py| { self.0 } }
    ),
    (
        PyEnumIntegerConstant,
        {
            value_from: i64, |self, _py| { self.value_from },
            value_to: i64, |self, _py| { self.value_to }
        }
    ),
    (
        PyBooleanConstant,
        { value: bool, |self, _py| { self.0 } }
    ),
    (
        PyNaNConstant,
        { value: f64, |self, _py| { f64::NAN } }
    ),
    (
        PyDriverConstant,
        { value: PyDriver, |self, _py| { self.0.clone().into() } }
    ),
    (
        PyRangeConstant,
        { value: f64, |self, _py| { self.0 } }
    ),
    (
        PyArrayConstant,
        { value: Vec<f64>, |self, _py| { self.0.clone() } }
    ),
    (
        PyStringConstant,
        { value: &str, |self, _py| { &self.0 } }
    ),
    (
        PyPlaceholderExpr,
        { name: &str, |self, _py| { &self.0 } }
    ),
    (
        PyVariableExpr,
        { name: &str, |self, _py| { &self.0 } }
    ),
);

#[pyfunction]
fn parse(py: Python<'_>, input: &str) -> PyResult<Py<PyAlpha>> {
    let parsed = parse_rust(input)
        .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;
    Py::new(py, PyAlpha(parsed))
}

#[pymodule]
#[pyo3(name = "core")]
pub fn fastplus(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyAlpha>()?;
    module.add_class::<PyAssignment>()?;
    module.add_class::<PyBinaryOp>()?;
    module.add_class::<PyUnaryOp>()?;
    module.add_class::<PyKwArg>()?;
    module.add_class::<PyDriver>()?;
    module.add_class::<PyExpression>()?;
    module.add_class::<PyTernaryExpr>()?;
    module.add_class::<PyBinaryExpr>()?;
    module.add_class::<PyUnaryExpr>()?;
    module.add_class::<PyOperationExpr>()?;
    module.add_class::<PyConstant>()?;
    module.add_class::<PyFloatConstant>()?;
    module.add_class::<PyNonNegativeFloatConstant>()?;
    module.add_class::<PyPositiveFloatConstant>()?;
    module.add_class::<PyRatioConstant>()?;
    module.add_class::<PyIntegerConstant>()?;
    module.add_class::<PyNonNegativeIntegerConstant>()?;
    module.add_class::<PyPositiveIntegerConstant>()?;
    module.add_class::<PyEnumIntegerConstant>()?;
    module.add_class::<PyBooleanConstant>()?;
    module.add_class::<PyNaNConstant>()?;
    module.add_class::<PyDriverConstant>()?;
    module.add_class::<PyRangeConstant>()?;
    module.add_class::<PyArrayConstant>()?;
    module.add_class::<PyStringConstant>()?;
    module.add_class::<PyPlaceholderExpr>()?;
    module.add_class::<PyVariableExpr>()?;
    module.add_class::<PyGroupExpr>()?;
    module.add_function(wrap_pyfunction!(parse, module)?)?;
    Ok(())
}
