use crate::ast::{
    BinaryOperator, Expression, Function, Literal, Mutability, Program, Statement, UnaryOperator,
    VariableType,
};
use std::collections::HashMap;

#[derive(Clone)]
struct Symbol {
    ty: VariableType,
    mutability: Mutability,
}

pub fn generate(program: &Program) -> String {
    let mut output = String::from(
        "#include <stdbool.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <math.h>\n\n",
    );
    let mut globals = HashMap::new();
    for declaration in &program.identity.variables {
        globals.insert(
            declaration.name.clone(),
            Symbol {
                ty: inferred_type(&declaration.value, &globals, &program.functions),
                mutability: declaration.mutability,
            },
        );
    }

    for function in &program.functions {
        output.push_str(&function_signature(function));
        output.push_str(";\n");
    }
    if !program.functions.is_empty() {
        output.push('\n');
    }

    for declaration in &program.identity.variables {
        output.push_str(&format!(
            "static {}{} {} = {};\n",
            const_prefix(declaration.mutability),
            c_type(&globals[&declaration.name].ty),
            declaration.name,
            emit_expression(&declaration.value, &globals, &program.functions),
        ));
    }
    if !program.identity.variables.is_empty() {
        output.push('\n');
    }

    for function in &program.functions {
        let mut symbols = globals.clone();
        for parameter in &function.parameters {
            symbols.insert(
                parameter.name.clone(),
                Symbol {
                    ty: parameter.ty.clone(),
                    mutability: Mutability::Constant,
                },
            );
        }
        output.push_str(&function_signature(function));
        output.push_str(" {\n");
        emit_statements(&function.body, &mut symbols, &program.functions, 1, &mut output);
        output.push_str("}\n\n");
    }

    output.push_str("int main(void) {\n");
    let mut symbols = globals;
    emit_statements(&program.main, &mut symbols, &program.functions, 1, &mut output);
    output.push_str("    return 0;\n}\n");
    output
}

fn function_signature(function: &Function) -> String {
    let return_type = function
        .return_type
        .as_ref()
        .map(c_type)
        .unwrap_or_else(|| "void".to_string());
    let parameters = if function.parameters.is_empty() {
        "void".to_string()
    } else {
        function
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}{} {}",
                    const_prefix(Mutability::Constant),
                    c_type(&parameter.ty),
                    parameter.name
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("{return_type} {}({parameters})", function.name)
}

fn c_type(ty: &VariableType) -> String {
    match ty {
        VariableType::Inferred => "int32_t".to_string(),
        VariableType::Integer { bits: 8, unsigned: false } => "int8_t".to_string(),
        VariableType::Integer { bits: 8, unsigned: true } => "uint8_t".to_string(),
        VariableType::Integer { bits: 16, unsigned: false } => "int16_t".to_string(),
        VariableType::Integer { bits: 16, unsigned: true } => "uint16_t".to_string(),
        VariableType::Integer { bits: 32, unsigned: false } => "int32_t".to_string(),
        VariableType::Integer { bits: 32, unsigned: true } => "uint32_t".to_string(),
        VariableType::Float { .. } => "float".to_string(),
        VariableType::String { .. } => "char *".to_string(),
        VariableType::Bool => "bool".to_string(),
        VariableType::Integer { .. } => "int32_t".to_string(),
    }
}

fn const_prefix(mutability: Mutability) -> &'static str {
    if mutability == Mutability::Constant {
        "const "
    } else {
        ""
    }
}

fn inferred_type(
    expression: &Expression,
    symbols: &HashMap<String, Symbol>,
    functions: &[Function],
) -> VariableType {
    match expression {
        Expression::Literal(Literal::Integer(_)) => {
            VariableType::Integer { bits: 32, unsigned: false }
        }
        Expression::Literal(Literal::Float(_)) => VariableType::Float { bits: 32 },
        Expression::Literal(Literal::String(_)) => VariableType::String { bits: 16 },
        Expression::Literal(Literal::Bool(_)) => VariableType::Bool,
        Expression::Variable(name) => symbols
            .get(name)
            .map(|symbol| symbol.ty.clone())
            .unwrap_or(VariableType::Inferred),
        Expression::Call { name, .. } => functions
            .iter()
            .find(|function| function.name == *name)
            .and_then(|function| function.return_type.clone())
            .unwrap_or(VariableType::Inferred),
        Expression::Unary { expression, .. } => inferred_type(expression, symbols, functions),
        Expression::Binary { left, operator, .. } => match operator {
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::Less
            | BinaryOperator::GreaterEqual
            | BinaryOperator::LessEqual
            | BinaryOperator::LogicalAnd
            | BinaryOperator::LogicalOr => VariableType::Bool,
            _ => inferred_type(left, symbols, functions),
        },
        Expression::If { then_value, .. } => inferred_type(then_value, symbols, functions),
        Expression::Introspection { kind, .. } => match kind {
            crate::ast::IntrospectionKind::Type
            | crate::ast::IntrospectionKind::Sign
            | crate::ast::IntrospectionKind::Identity => VariableType::String { bits: 16 },
            crate::ast::IntrospectionKind::BitSize => {
                VariableType::Integer { bits: 8, unsigned: false }
            }
            crate::ast::IntrospectionKind::IsMut => VariableType::Bool,
        },
    }
}

fn emit_statements(
    statements: &[Statement],
    symbols: &mut HashMap<String, Symbol>,
    functions: &[Function],
    indent: usize,
    output: &mut String,
) {
    let pad = "    ".repeat(indent);
    for statement in statements {
        match statement {
            Statement::Variable(declaration) => {
                let ty = inferred_type(&declaration.value, symbols, functions);
                output.push_str(&format!(
                    "{pad}{}{} {} = {};\n",
                    const_prefix(declaration.mutability),
                    c_type(&ty),
                    declaration.name,
                    emit_expression(&declaration.value, symbols, functions),
                ));
                symbols.insert(
                    declaration.name.clone(),
                    Symbol { ty, mutability: declaration.mutability },
                );
            }
            Statement::Assignment { name, value } => output.push_str(&format!(
                "{pad}{name} = {};\n",
                emit_expression(value, symbols, functions),
            )),
            Statement::Display(expressions) => {
                for (index, expression) in expressions.iter().enumerate() {
                    if index > 0 {
                        output.push_str(&format!("{pad}printf(\" \");\n"));
                    }
                    let ty = inferred_type(expression, symbols, functions);
                    let code = emit_expression(expression, symbols, functions);
                    match ty {
                        VariableType::String { .. } => {
                            output.push_str(&format!("{pad}printf(\"%s\", {code});\n"))
                        }
                        VariableType::Float { .. } => output.push_str(&format!(
                            "{pad}printf(\"%g\", (double)({code}));\n"
                        )),
                        VariableType::Bool => output.push_str(&format!(
                            "{pad}printf(\"%s\", ({code}) ? \"true\" : \"false\");\n"
                        )),
                        _ => output.push_str(&format!(
                            "{pad}printf(\"%lld\", (long long)({code}));\n"
                        )),
                    }
                }
                output.push_str(&format!("{pad}printf(\"\\n\");\n"));
            }
            Statement::Expression(expression) => output.push_str(&format!(
                "{pad}{};\n",
                emit_expression(expression, symbols, functions),
            )),
            Statement::Return(value) => {
                if let Some(value) = value {
                    output.push_str(&format!(
                        "{pad}return {};\n",
                        emit_expression(value, symbols, functions),
                    ));
                } else {
                    output.push_str(&format!("{pad}return;\n"));
                }
            }
            Statement::If { condition, then_branch, else_branch } => {
                output.push_str(&format!(
                    "{pad}if ({}) {{\n",
                    emit_expression(condition, symbols, functions),
                ));
                let mut then_symbols = symbols.clone();
                emit_statements(then_branch, &mut then_symbols, functions, indent + 1, output);
                output.push_str(&format!("{pad}}}"));
                if let Some(else_branch) = else_branch {
                    output.push_str(" else {\n");
                    let mut else_symbols = symbols.clone();
                    emit_statements(else_branch, &mut else_symbols, functions, indent + 1, output);
                    output.push_str(&format!("{pad}}}"));
                }
                output.push('\n');
            }
            Statement::Chunk { body, .. } => {
                output.push_str(&format!("{pad}{{\n"));
                let mut child_symbols = symbols.clone();
                emit_statements(body, &mut child_symbols, functions, indent + 1, output);
                output.push_str(&format!("{pad}}}\n"));
            }
        }
    }
}

fn emit_expression(
    expression: &Expression,
    symbols: &HashMap<String, Symbol>,
    functions: &[Function],
) -> String {
    match expression {
        Expression::Literal(Literal::Integer(value)) => value.to_string(),
        Expression::Literal(Literal::Float(value)) => value.to_string(),
        Expression::Literal(Literal::String(value)) => format!("{value:?}"),
        Expression::Literal(Literal::Bool(value)) => value.to_string(),
        Expression::Variable(name) => name.clone(),
        Expression::Introspection { kind, argument } => {
            let name = if let Expression::Variable(name) = argument.as_ref() {
                name
            } else {
                ""
            };
            let symbol = symbols.get(name);
            match kind {
                crate::ast::IntrospectionKind::Type => format!(
                    "{:?}",
                    symbol.map(|s| type_label(&s.ty)).unwrap_or("unknown")
                ),
                crate::ast::IntrospectionKind::Sign => format!(
                    "{:?}",
                    symbol.map(|s| sign_label(&s.ty)).unwrap_or("not_applicable")
                ),
                crate::ast::IntrospectionKind::BitSize => symbol
                    .map(|s| bit_width(&s.ty).to_string())
                    .unwrap_or_else(|| "0".to_string()),
                crate::ast::IntrospectionKind::IsMut => symbol
                    .map(|s| (s.mutability == Mutability::Mutable).to_string())
                    .unwrap_or_else(|| "false".to_string()),
                crate::ast::IntrospectionKind::Identity => format!(
                    "{:?}",
                    symbol
                        .map(|s| format!(
                            "{{ type: {}, sign: {}, bits: {}, mut: {} }}",
                            type_label(&s.ty),
                            sign_label(&s.ty),
                            bit_width(&s.ty),
                            s.mutability == Mutability::Mutable
                        ))
                        .unwrap_or_default()
                ),
            }
        }
        Expression::Unary { operator, expression } => format!(
            "({}{})",
            match operator {
                UnaryOperator::Not => "!",
                UnaryOperator::Negate => "-",
                UnaryOperator::BitNot => "~",
            },
            emit_expression(expression, symbols, functions),
        ),
        Expression::Binary { left, operator: BinaryOperator::Power, right } => format!(
            "pow((double)({}), (double)({}))",
            emit_expression(left, symbols, functions),
            emit_expression(right, symbols, functions),
        ),
        Expression::Binary { left, operator, right } => format!(
            "({} {} {})",
            emit_expression(left, symbols, functions),
            operator_symbol(*operator),
            emit_expression(right, symbols, functions),
        ),
        Expression::Call { name, arguments } => format!(
            "{}({})",
            name,
            arguments
                .iter()
                .map(|argument| emit_expression(argument, symbols, functions))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Expression::If { condition, then_value, else_value } => format!(
            "({} ? {} : {})",
            emit_expression(condition, symbols, functions),
            emit_expression(then_value, symbols, functions),
            emit_expression(else_value, symbols, functions),
        ),
    }
}

fn operator_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Remainder => "%",
        BinaryOperator::Power => unreachable!("la potenza e' emessa con pow"),
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::Greater => ">",
        BinaryOperator::Less => "<",
        BinaryOperator::GreaterEqual => ">=",
        BinaryOperator::LessEqual => "<=",
        BinaryOperator::LogicalAnd => "&&",
        BinaryOperator::LogicalOr => "||",
        BinaryOperator::BitAnd => "&",
        BinaryOperator::BitOr => "|",
        BinaryOperator::BitXor => "^",
        BinaryOperator::ShiftLeft => "<<",
        BinaryOperator::ShiftRight => ">>",
    }
}

fn type_label(ty: &VariableType) -> &'static str {
    match ty {
        VariableType::Inferred | VariableType::Integer { bits: 32, unsigned: false } => "32bit",
        VariableType::Integer { bits: 8, unsigned: false } => "8bit",
        VariableType::Integer { bits: 16, unsigned: false } => "16bit",
        VariableType::Integer { bits: 8, unsigned: true } => "unsigned.8bit",
        VariableType::Integer { bits: 16, unsigned: true } => "unsigned.16bit",
        VariableType::Integer { bits: 32, unsigned: true } => "unsigned.32bit",
        VariableType::Float { .. } => "32bit.float",
        VariableType::String { .. } => "16bit.string",
        VariableType::Bool => "bool",
        VariableType::Integer { .. } => "32bit",
    }
}

fn sign_label(ty: &VariableType) -> &'static str {
    match ty {
        VariableType::Integer { unsigned: true, .. } => "unsigned",
        VariableType::Integer { .. } | VariableType::Inferred | VariableType::Float { .. } => "signed",
        _ => "not_applicable",
    }
}

fn bit_width(ty: &VariableType) -> u8 {
    match ty {
        VariableType::Integer { bits, .. }
        | VariableType::Float { bits }
        | VariableType::String { bits } => *bits,
        VariableType::Inferred => 32,
        VariableType::Bool => 1,
    }
}
