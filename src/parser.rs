//！ Parser
/// build_* -> struct
/// parse_* -> enum
use crate::ast::{Alpha, Assignment, BinaryOp, Expression, FastPlusParser, Rule, UnaryOp};
use crate::field::{Constant, Field};
use crate::operator::get_operator;
use pest::Parser;
use pest::Position;
use pest::error::{Error as PestError, ErrorVariant::CustomError};
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use std::collections::{HashMap, hash_map::Entry};

pub fn parse(input: &str) -> Result<Alpha, PestError<Rule>> {
    let input = input.trim_end();
    check_parentheses(input)?;
    let mut pairs = FastPlusParser::parse(Rule::start, input)?;
    let start = pairs.next().unwrap();
    let alpha = start.into_inner().next().unwrap();
    build_alpha(alpha)
}

fn check_parentheses(input: &str) -> Result<(), PestError<Rule>> {
    let mut parentheses = Vec::new();
    let mut chars = input.char_indices().peekable();
    let mut quote = None;
    let mut line_comment = false;
    let mut block_comment = false;

    while let Some((index, ch)) = chars.next() {
        if line_comment {
            if ch == '\n' {
                line_comment = false;
            }
            continue;
        }
        if block_comment {
            if ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if let Some(quote_char) = quote {
            if ch == '\\' {
                chars.next();
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '/' if chars.peek().is_some_and(|(_, next)| *next == '/') => {
                chars.next();
                line_comment = true;
            }
            '/' if chars.peek().is_some_and(|(_, next)| *next == '*') => {
                chars.next();
                block_comment = true;
            }
            '(' => parentheses.push(index),
            ')' if parentheses.pop().is_none() => {
                return Err(PestError::new_from_pos(
                    CustomError {
                        message: "多余的右括号 `)`".to_string(),
                    },
                    Position::new(input, index).unwrap(),
                ));
            }
            _ => {}
        }
    }

    if let Some(open_index) = parentheses.last().copied() {
        let (line, column) = Position::new(input, open_index).unwrap().line_col();
        return Err(PestError::new_from_pos(
            CustomError {
                message: format!("缺少右括号 `)`（左括号位于第 {line} 行第 {column} 列）"),
            },
            Position::new(input, input.len()).unwrap(),
        ));
    }

    Ok(())
}

fn build_alpha(pair: Pair<Rule>) -> Result<Alpha, PestError<Rule>> {
    let mut span = pair.as_span();
    match pair.as_rule() {
        Rule::alpha => {
            let pair = pair.into_inner();
            let mut assignments = Vec::new();
            let mut signal = None;

            for p in pair {
                span = p.as_span();
                match p.as_rule() {
                    Rule::assignment => assignments.push(build_assignment(p)?),
                    Rule::expression => signal = Some(parse_expression(p)?),
                    other => {
                        return Err(PestError::new_from_span(
                            CustomError {
                                message: format!("build_alpha unexpected rule: {:#?}", other),
                            },
                            span,
                        ));
                    }
                }
            }
            let signal = signal.ok_or(PestError::new_from_span(
                CustomError {
                    message: "因子只有赋值语句, 缺少信号表达式".to_string(),
                },
                span,
            ))?;
            if let Some(signal_type) = infer_field(&signal)
                && !Field::Matrix.fit(&signal_type)
            {
                return Err(PestError::new_from_span(
                    CustomError {
                        message: format!(
                            "signal must be Matrix-compatible, but got {signal_type:?}"
                        ),
                    },
                    span,
                ));
            }
            Ok(Alpha::new(assignments, signal))
        }
        other => Err(PestError::new_from_span(
            CustomError {
                message: format!("build_alpha unexpected rule: {:#?}", other),
            },
            span,
        )),
    }
}

fn build_assignment(pair: Pair<Rule>) -> Result<Assignment, PestError<Rule>> {
    let mut assignment = pair.into_inner();
    let variable = assignment.next().unwrap();
    let value = assignment.next().unwrap();

    if let Rule::expression = value.as_rule() {
        Ok(Assignment::new(
            variable.to_string(),
            parse_expression(value)?,
        ))
    } else {
        Err(PestError::new_from_span(
            CustomError {
                message: format!("build_assignment unexpected rule: {:#?}", value.as_rule()),
            },
            value.as_span(),
        ))
    }
}

fn parse_expression(pair: Pair<Rule>) -> Result<Expression, PestError<Rule>> {
    match pair.as_rule() {
        Rule::expression => {
            // expression = { pratt_expr ~ ternary_tail? }
            let mut inner = pair.into_inner();
            let pratt_expr = inner.next().unwrap();

            let expr = parse_pratt_expression(pratt_expr)?;

            if let Some(ternary_tail) = inner.next() {
                match ternary_tail.as_rule() {
                    Rule::ternary_tail => {
                        let mut tail_inner = ternary_tail.into_inner();
                        let then_branch = tail_inner.next().unwrap();
                        let else_branch = tail_inner.next().unwrap();
                        Ok(Expression::Ternary {
                            cond: Box::new(expr),
                            then_branch: Box::new(parse_expression(then_branch)?),
                            else_branch: Box::new(parse_expression(else_branch)?),
                        })
                    }
                    other => Err(PestError::new_from_span(
                        CustomError {
                            message: format!(
                                "parse_expression(ternay_tail) unexpected rule: {:#?}",
                                other
                            ),
                        },
                        ternary_tail.as_span(),
                    )),
                }
            } else {
                Ok(expr)
            }
        }
        Rule::pratt_expr => parse_pratt_expression(pair),
        other => Err(PestError::new_from_span(
            CustomError {
                message: format!("parse_expression unexpected rule: {:#?}", other),
            },
            pair.as_span(),
        )),
    }
}

fn parse_pratt_expression(pair: Pair<Rule>) -> Result<Expression, PestError<Rule>> {
    let pratt_parser = PrattParser::new()
        .op(Op::prefix(Rule::prefix))
        .op(Op::infix(Rule::or_op, Assoc::Left))
        .op(Op::infix(Rule::and_op, Assoc::Left))
        .op(Op::infix(Rule::cmp_op, Assoc::Left))
        .op(Op::infix(Rule::add_op, Assoc::Left))
        .op(Op::infix(Rule::mul_op, Assoc::Left));

    let expression_pairs = pair.into_inner();

    pratt_parser
        .map_primary(|primary| match primary.as_rule() {
            Rule::primary => {
                let inner = primary.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::operation => {
                        let operation_span = inner.as_span();
                        let mut operation = inner.into_inner();
                        // op(args*, kwargs*)
                        let op = operation.next().unwrap().to_string();
                        let mut pos_args = Vec::<Expression>::new();
                        let mut kw_args = HashMap::new();

                        let args = operation.next().unwrap().into_inner();
                        for arg in args {
                            match arg.as_rule() {
                                Rule::posarg => pos_args
                                    .push(parse_expression(arg.into_inner().next().unwrap())?),
                                Rule::kwarg => {
                                    let arg_span = arg.as_span();
                                    let mut kwarg = arg.into_inner();
                                    let name = kwarg.next().unwrap().to_string();
                                    let value = kwarg.next().unwrap();
                                    if let Entry::Occupied(_) = kw_args.entry(name.clone()) {
                                        return Err(PestError::new_from_span(
                                            CustomError {
                                                message: format!("重复的关键字参数 `{name}`"),
                                            },
                                            arg_span,
                                        ));
                                    }
                                    let value =
                                        Constant::from(value.into_inner().next().unwrap().as_str());
                                    kw_args.insert(name, value);
                                }
                                other => {
                                    return Err(PestError::new_from_span(
                                        CustomError {
                                            message: format!(
                                                "parse_pratt_expression unexpected rule: {:#?}",
                                                other
                                            ),
                                        },
                                        arg.as_span(),
                                    ));
                                }
                            }
                        }

                        if let Err(message) = check_operation(&op, &pos_args, &kw_args) {
                            return Err(PestError::new_from_span(
                                CustomError { message },
                                operation_span,
                            ));
                        }

                        Ok(Expression::Operation {
                            op,
                            pos_args,
                            kw_args,
                        })
                    }
                    Rule::PLACEHOLDER => Ok(Expression::Placeholder(inner.as_str().to_string())),
                    Rule::VARIABLE => Ok(Expression::Variable(inner.as_str().to_string())),
                    Rule::CONSTANT => Ok(Expression::Constant(Constant::from(
                        inner.into_inner().next().unwrap().as_str(),
                    ))),
                    Rule::expression => Ok(Expression::Group(Box::new(parse_expression(inner)?))),
                    other => Err(PestError::new_from_span(
                        CustomError {
                            message: format!(
                                "parse_pratt_expression(primary) unexpected rule: {:#?}",
                                other
                            ),
                        },
                        inner.as_span(),
                    )),
                }
            }
            other => Err(PestError::new_from_span(
                CustomError {
                    message: format!(
                        "parse_pratt_expression(primary) unexpected rule: {:#?}",
                        other
                    ),
                },
                primary.as_span(),
            )),
        })
        .map_prefix(|op, rhs| {
            let expr = rhs?;
            let unary = match op.as_rule() {
                Rule::prefix => match op.as_str() {
                    "+" => UnaryOp::Positive,
                    "-" => UnaryOp::Negative,
                    "!" => UnaryOp::Not,
                    other => {
                        return Err(PestError::new_from_span(
                            CustomError {
                                message: format!(
                                    "parse_pratt_expression(prefix) unexpected prefix: {:#?}",
                                    other
                                ),
                            },
                            op.as_span(),
                        ));
                    }
                },
                other => {
                    return Err(PestError::new_from_span(
                        CustomError {
                            message: format!(
                                "parse_pratt_expression(prefix) unexpected rule: {:#?}",
                                other
                            ),
                        },
                        op.as_span(),
                    ));
                }
            };

            Ok(Expression::Unary {
                op: unary,
                expr: Box::new(expr),
            })
        })
        .map_infix(|lhs, op, rhs| {
            let left = lhs?;
            let right = rhs?;
            let binary = match op.as_str() {
                "||" => BinaryOp::Or,
                "&&" => BinaryOp::And,
                "==" => BinaryOp::Equal,
                "!=" => BinaryOp::NotEqual,
                ">=" => BinaryOp::GreaterEqual,
                "<=" => BinaryOp::LessEqual,
                ">" => BinaryOp::Greater,
                "<" => BinaryOp::Less,
                "+" => BinaryOp::Add,
                "-" => BinaryOp::Subtract,
                "*" => BinaryOp::Multiply,
                "/" => BinaryOp::Divide,
                other => {
                    return Err(PestError::new_from_span(
                        CustomError {
                            message: format!(
                                "parse_pratt_expression(infox) unexpected infix: {:#?}",
                                other
                            ),
                        },
                        op.as_span(),
                    ));
                }
            };

            Ok(Expression::Binary {
                left: Box::new(left),
                op: binary,
                right: Box::new(right),
            })
        })
        .parse(expression_pairs)
}

fn check_operation(
    op: &str,
    pos_args: &[Expression],
    kw_args: &HashMap<String, Constant>,
) -> Result<(), String> {
    let Some(operator) = get_operator(op) else {
        return Ok(());
    };

    let fields = pos_args
        .iter()
        .enumerate()
        .map(|(index, expression)| {
            infer_field(expression).unwrap_or_else(|| {
                if operator.nary == -1 {
                    operator.pos_args[0].clone()
                } else {
                    operator
                        .pos_args
                        .get(index)
                        .or_else(|| operator.pos_args.first())
                        .cloned()
                        .unwrap_or(Field::Matrix)
                }
            })
        })
        .collect::<Vec<_>>();

    operator
        .apply(&fields, kw_args)
        .map(|_| ())
        .map_err(|error| format!("operator `{op}` check failed: {error:?}"))
}

fn infer_field(expression: &Expression) -> Option<Field> {
    match expression {
        Expression::Constant(value) => Some(Field::Constant(value.clone())),
        Expression::Group(expression) => infer_field(expression),
        Expression::Operation { op, .. } => {
            get_operator(op).map(|operator| operator.return_type.clone())
        }
        Expression::Ternary { .. } | Expression::Binary { .. } | Expression::Unary { .. } => {
            Some(Field::Matrix)
        }
        Expression::Variable(_) | Expression::Placeholder(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parse_success() {
        let inputs = [
            "returns",
            "a=1;returns",
            "x ? y : z",
            "foo(<x/>)",
            "-(1 + 2) * 3",
            "  returns  \n\t",
        ];

        for input in inputs {
            assert!(parse(input).is_ok(), "expected `{input}` to parse");
        }
    }

    #[test]
    fn signal_must_be_matrix_compatible() {
        assert!(parse("group_rank(x, industry)").is_ok());
        assert!(parse("vec_avg(<prices/>)").is_ok());
        assert!(parse("42").is_ok());
        assert!(parse("bucket(rank(x), range='0, 1, 0.1')").is_err());
        assert!(parse("bucket(rank(x), buckets='0, 1')").is_err());
    }

    #[test]
    fn parse_failure() {
        let inputs = [
            "a=1;",
            "a=1;b=2;c=foo(x, y, z=3)",
            "foo(z=1, x)",
            "1abc",
            "x ? y",
            "foo(,x)",
            "foo(x=1, x=2)",
        ];

        for input in inputs {
            assert!(parse(input).is_err(), "expected `{input}` to fail");
        }
    }

    #[test]
    fn checks_known_operations_with_unknown_inputs() {
        assert!(parse("divide(x, y)").is_ok());
        assert!(parse("group_neutralize(x, industry)").is_ok());
        assert!(parse("divide(add(x, y), z)").is_ok());
        assert!(parse("custom_operator(1, 2)").is_ok());

        assert!(parse("divide('foo', 2)").is_err());
        assert!(parse("divide(x)").is_err());
    }

    #[test]
    fn collects_typed_fields_and_operator_names() {
        let alpha = parse("a=ts_delay(close, 5);group_rank(a, industry)").unwrap();
        let fields = alpha.fields();
        assert_eq!(fields.matrix, vec!["close"]);
        assert_eq!(fields.group, vec!["industry"]);
        assert!(fields.vector.is_empty());
        assert_eq!(alpha.operators(), vec!["ts_delay", "group_rank"]);

        let alpha = parse("vec_avg(<prices/>)").unwrap();
        let fields = alpha.fields();
        assert_eq!(fields.vector, vec!["<prices/>"]);
        assert!(fields.matrix.is_empty());
    }

    #[test]
    fn reports_missing_closing_parenthesis() {
        let error = parse("multiply(rank(abs(zscore(<score/>)))").unwrap_err();
        assert!(error.to_string().contains("缺少右括号 `)`"));
    }
}
