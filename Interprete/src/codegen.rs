use crate::ast::{
    BinaryOperator, EnumDefinition, Expression, Function, Literal, Mutability, Program, Statement,
    StructDefinition, UnaryOperator, VariableType,
};
use std::collections::HashMap;

#[derive(Clone)]
struct Symbol {
    ty: VariableType,
    mutability: Mutability,
}

pub fn generate(program: &Program) -> String {
    let mut output = String::from(
        "#include <stdbool.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <math.h>\n#include <stddef.h>\n#define LDC_PUSH(a,n,c,d,v) do { if (!(d)) { fprintf(stderr, \"push requires a dynamic array\\n\"); exit(1); } if ((n) >= (c)) { fprintf(stderr, \"array capacity exceeded\\n\"); exit(1); } (a)[(n)++] = (v); } while (0)\n#define LDC_INDEX(a,i,n) (((i) < 0 || (size_t)(i) >= (n)) ? (fprintf(stderr, \"array index out of bounds\\n\"), exit(1), (a)[0]) : (a)[i])\n#define LDC_SET(a,i,n,v) do { if ((i) < 0 || (size_t)(i) >= (n)) { fprintf(stderr, \"array index out of bounds\\n\"); exit(1); } (a)[i] = (v); } while (0)\n#define LDC_SET_FIELD(a,i,n,f,v) do { if ((i) < 0 || (size_t)(i) >= (n)) { fprintf(stderr, \"array index out of bounds\\n\"); exit(1); } (a)[i].f = (v); } while (0)\n#define LDC_POP(a,n) ((n) > 0 ? (a)[--(n)] : (fprintf(stderr, \"pop on empty array\\n\"), exit(1), (a)[0]))\n\n",
    );
    emit_type_definitions(program, &mut output);
    let mut globals = HashMap::new();
    for declaration in &program.identity.variables {
        globals.insert(
            declaration.name.clone(),
            Symbol {
                ty: if matches!(declaration.ty, VariableType::Inferred) {
                    inferred_type(
                        &declaration.value,
                        &globals,
                        &program.functions,
                        &program.structs,
                        &program.enums,
                    )
                } else {
                    declaration.ty.clone()
                },
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
        if let VariableType::Array {
            element,
            capacity,
            dynamic,
        } = &globals[&declaration.name].ty
        {
            let values = match &declaration.value {
                Expression::Array(values) => values
                    .iter()
                    .map(|value| {
                        emit_expression(
                            value,
                            &globals,
                            &program.functions,
                            &program.structs,
                            &program.enums,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => "0".to_string(),
            };
            output.push_str(&format!(
                "static {}{} {}[{}] = {{{}}};\n",
                composite_const_prefix(declaration.mutability, &globals[&declaration.name].ty),
                c_type(element),
                declaration.name,
                capacity,
                values
            ));
            let length = match &declaration.value {
                Expression::Array(values) => values.len(),
                _ if !dynamic => *capacity,
                _ => 0,
            };
            output.push_str(&format!(
                "static size_t {}_len = {};\n",
                declaration.name, length
            ));
        } else {
            output.push_str(&format!(
                "static {}{} {} = {};\n",
                composite_const_prefix(declaration.mutability, &declaration.ty),
                c_type(&globals[&declaration.name].ty),
                declaration.name,
                emit_expression(
                    &declaration.value,
                    &globals,
                    &program.functions,
                    &program.structs,
                    &program.enums
                )
            ));
        }
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
        emit_statements(
            &function.body,
            &mut symbols,
            &program.functions,
            &program.structs,
            &program.enums,
            1,
            &mut output,
        );
        output.push_str("}\n\n");
    }

    output.push_str("int main(void) {\n");
    let mut symbols = globals;
    emit_statements(
        &program.main,
        &mut symbols,
        &program.functions,
        &program.structs,
        &program.enums,
        1,
        &mut output,
    );
    output.push_str("    return 0;\n}\n");
    output
}

fn emit_type_definitions(program: &Program, output: &mut String) {
    for definition in &program.structs {
        output.push_str(&format!("typedef struct {} {{\n", definition.name));
        for field in &definition.fields {
            output.push_str(&format!(
                "    {};\n",
                c_declaration(&field.ty, &field.name, field.mutability)
            ));
        }
        output.push_str(&format!("}} {};\n\n", definition.name));
    }
    for definition in &program.enums {
        output.push_str(&format!(
            "typedef struct {} {{ int tag; union {{\n",
            definition.name
        ));
        if definition
            .variants
            .iter()
            .all(|variant| variant.payload.is_empty())
        {
            output.push_str("    uint8_t empty;\n");
        }
        for (index, variant) in definition.variants.iter().enumerate() {
            if variant.payload.is_empty() {
                continue;
            }
            output.push_str(&format!("    struct {{ "));
            for (field_index, ty) in variant.payload.iter().enumerate() {
                output.push_str(&format!("{} f{}; ", c_type(ty), field_index));
            }
            output.push_str(&format!("}} v{index};\n"));
        }
        output.push_str(&format!("}} data; }} {};\n\n", definition.name));
    }
}

fn c_declaration(ty: &VariableType, name: &str, mutability: Mutability) -> String {
    let prefix = match ty {
        VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_) => "",
        VariableType::Array { element, .. }
            if matches!(
                element.as_ref(),
                VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_)
            ) =>
        {
            ""
        }
        _ => const_prefix(mutability),
    };
    match ty {
        VariableType::Array {
            element, capacity, ..
        } => format!("{prefix}{} {}[{}]", c_type(element), name, capacity),
        _ => format!("{prefix}{} {}", c_type(ty), name),
    }
}

fn composite_const_prefix(mutability: Mutability, ty: &VariableType) -> &'static str {
    let composite = match ty {
        VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_) => true,
        VariableType::Array { element, .. } => matches!(
            element.as_ref(),
            VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_)
        ),
        _ => false,
    };
    if composite {
        ""
    } else {
        const_prefix(mutability)
    }
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
        VariableType::Integer {
            bits: 8,
            unsigned: false,
        } => "int8_t".to_string(),
        VariableType::Integer {
            bits: 8,
            unsigned: true,
        } => "uint8_t".to_string(),
        VariableType::Integer {
            bits: 16,
            unsigned: false,
        } => "int16_t".to_string(),
        VariableType::Integer {
            bits: 16,
            unsigned: true,
        } => "uint16_t".to_string(),
        VariableType::Integer {
            bits: 32,
            unsigned: false,
        } => "int32_t".to_string(),
        VariableType::Integer {
            bits: 32,
            unsigned: true,
        } => "uint32_t".to_string(),
        VariableType::Float { .. } => "float".to_string(),
        VariableType::String { .. } => "char *".to_string(),
        VariableType::Bool => "bool".to_string(),
        VariableType::Array { element, .. } => c_type(element),
        VariableType::Named(name) | VariableType::Struct(name) | VariableType::Enum(name) => {
            name.clone()
        }
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
    structs: &[StructDefinition],
    enums: &[EnumDefinition],
) -> VariableType {
    match expression {
        Expression::Literal(Literal::Integer(_)) => VariableType::Integer {
            bits: 32,
            unsigned: false,
        },
        Expression::Literal(Literal::Float(_)) => VariableType::Float { bits: 32 },
        Expression::Literal(Literal::String(_)) => VariableType::String { bits: 16 },
        Expression::Literal(Literal::Bool(_)) => VariableType::Bool,
        Expression::Literal(Literal::Struct { name, .. } | Literal::Enum { name, .. }) => {
            VariableType::Named(name.clone())
        }
        Expression::Literal(Literal::Array(values)) => VariableType::Array {
            element: Box::new(
                values
                    .first()
                    .map(|value| match value {
                        Literal::Integer(_) => VariableType::Integer {
                            bits: 32,
                            unsigned: false,
                        },
                        Literal::Float(_) => VariableType::Float { bits: 32 },
                        Literal::String(_) => VariableType::String { bits: 16 },
                        Literal::Bool(_) => VariableType::Bool,
                        Literal::Array(_) => VariableType::Inferred,
                        Literal::Struct { name, .. } | Literal::Enum { name, .. } => {
                            VariableType::Named(name.clone())
                        }
                    })
                    .unwrap_or(VariableType::Inferred),
            ),
            capacity: values.len(),
            dynamic: false,
        },
        Expression::Array(values) => VariableType::Array {
            element: Box::new(
                values
                    .first()
                    .map(|first| inferred_type(first, symbols, functions, structs, enums))
                    .unwrap_or(VariableType::Inferred),
            ),
            capacity: values.len(),
            dynamic: false,
        },
        Expression::Index { array, .. } => {
            match inferred_type(array, symbols, functions, structs, enums) {
                VariableType::Array { element, .. } => *element,
                other => other,
            }
        }
        Expression::StructLiteral { name, .. } | Expression::DefaultStruct(name) => {
            VariableType::Named(name.clone())
        }
        Expression::EnumVariant { name, .. } => VariableType::Named(name.clone()),
        Expression::Field { value, name } => {
            match inferred_type(value, symbols, functions, structs, enums) {
                VariableType::Named(type_name) | VariableType::Struct(type_name) => structs
                    .iter()
                    .find(|definition| definition.name == type_name)
                    .and_then(|definition| {
                        definition.fields.iter().find(|field| field.name == *name)
                    })
                    .map(|field| field.ty.clone())
                    .unwrap_or(VariableType::Inferred),
                _ => VariableType::Inferred,
            }
        }
        Expression::Variable(name) => symbols
            .get(name)
            .map(|symbol| symbol.ty.clone())
            .unwrap_or(VariableType::Inferred),
        Expression::Call { name, .. } => functions
            .iter()
            .find(|function| function.name == *name)
            .and_then(|function| function.return_type.clone())
            .unwrap_or(VariableType::Inferred),
        Expression::Unary { expression, .. } => {
            inferred_type(expression, symbols, functions, structs, enums)
        }
        Expression::Binary { left, operator, .. } => match operator {
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::Less
            | BinaryOperator::GreaterEqual
            | BinaryOperator::LessEqual
            | BinaryOperator::LogicalAnd
            | BinaryOperator::LogicalOr => VariableType::Bool,
            _ => inferred_type(left, symbols, functions, structs, enums),
        },
        Expression::If { then_value, .. } => {
            inferred_type(then_value, symbols, functions, structs, enums)
        }
        Expression::Introspection { kind, .. } => match kind {
            crate::ast::IntrospectionKind::Type
            | crate::ast::IntrospectionKind::Sign
            | crate::ast::IntrospectionKind::Identity => VariableType::String { bits: 16 },
            crate::ast::IntrospectionKind::BitSize => VariableType::Integer {
                bits: 8,
                unsigned: false,
            },
            crate::ast::IntrospectionKind::IsMut => VariableType::Bool,
        },
    }
}

fn emit_statements(
    statements: &[Statement],
    symbols: &mut HashMap<String, Symbol>,
    functions: &[Function],
    structs: &[StructDefinition],
    enums: &[EnumDefinition],
    indent: usize,
    output: &mut String,
) {
    let pad = "    ".repeat(indent);
    for statement in statements {
        match statement {
            Statement::Variable(declaration) => {
                let ty = inferred_type(&declaration.value, symbols, functions, structs, enums);
                let array_type = match &declaration.ty {
                    VariableType::Array {
                        element,
                        capacity,
                        dynamic,
                    } => Some((element.as_ref(), *capacity, *dynamic)),
                    VariableType::Inferred => match &declaration.value {
                        Expression::Array(values) => Some((
                            match &ty {
                                VariableType::Array { element, .. } => element.as_ref(),
                                _ => &VariableType::Inferred,
                            },
                            values.len(),
                            false,
                        )),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((element, capacity, dynamic)) = array_type {
                    let init = match &declaration.value {
                        Expression::Array(values) => values
                            .iter()
                            .map(|value| emit_expression(value, symbols, functions, structs, enums))
                            .collect::<Vec<_>>()
                            .join(", "),
                        _ => "0".to_string(),
                    };
                    output.push_str(&format!(
                        "{pad}{}{} {}[{}] = {{{}}};\n",
                        composite_const_prefix(
                            declaration.mutability,
                            &VariableType::Array {
                                element: Box::new(element.clone()),
                                capacity,
                                dynamic
                            }
                        ),
                        c_type(element),
                        declaration.name,
                        capacity,
                        init
                    ));
                    let count = match &declaration.value {
                        Expression::Array(values) => values.len(),
                        _ if !dynamic => capacity,
                        _ => 0,
                    };
                    output.push_str(&format!(
                        "{pad}size_t {}_len = {};\n",
                        declaration.name, count
                    ));
                    symbols.insert(
                        declaration.name.clone(),
                        Symbol {
                            ty: VariableType::Array {
                                element: Box::new(element.clone()),
                                capacity,
                                dynamic,
                            },
                            mutability: declaration.mutability,
                        },
                    );
                    continue;
                }
                output.push_str(&format!(
                    "{pad}{}{} {} = {};\n",
                    const_prefix(declaration.mutability),
                    c_type(&ty),
                    declaration.name,
                    emit_expression(&declaration.value, symbols, functions, structs, enums),
                ));
                symbols.insert(
                    declaration.name.clone(),
                    Symbol {
                        ty,
                        mutability: declaration.mutability,
                    },
                );
            }
            Statement::Assignment { name, value } => output.push_str(&format!(
                "{pad}{name} = {};\n",
                emit_expression(value, symbols, functions, structs, enums),
            )),
            Statement::IndexAssignment { name, index, value } => {
                let length = array_length(name, symbols);
                output.push_str(&format!(
                    "{pad}LDC_SET({name}, {}, {length}, {});\n",
                    emit_expression(index, symbols, functions, structs, enums),
                    emit_expression(value, symbols, functions, structs, enums)
                ));
            }
            Statement::FieldAssignment { target, value } => {
                if let Expression::Field {
                    value: base,
                    name: field,
                } = target
                {
                    if let Expression::Index { array, index } = base.as_ref() {
                        if let Expression::Variable(array_name) = array.as_ref() {
                            output.push_str(&format!(
                                "{pad}LDC_SET_FIELD({array_name}, {}, {}, {field}, {});\n",
                                emit_expression(index, symbols, functions, structs, enums),
                                array_length(array_name, symbols),
                                emit_expression(value, symbols, functions, structs, enums)
                            ));
                            continue;
                        }
                    }
                }
                output.push_str(&format!(
                    "{pad}{} = {};\n",
                    emit_expression(target, symbols, functions, structs, enums),
                    emit_expression(value, symbols, functions, structs, enums)
                ));
            }
            Statement::Display(expressions) => {
                for (index, expression) in expressions.iter().enumerate() {
                    if index > 0 {
                        output.push_str(&format!("{pad}printf(\" \");\n"));
                    }
                    let ty = inferred_type(expression, symbols, functions, structs, enums);
                    let code = emit_expression(expression, symbols, functions, structs, enums);
                    match ty {
                        VariableType::String { .. } => {
                            output.push_str(&format!("{pad}printf(\"%s\", {code});\n"))
                        }
                        VariableType::Float { .. } => {
                            output.push_str(&format!("{pad}printf(\"%g\", (double)({code}));\n"))
                        }
                        VariableType::Bool => output.push_str(&format!(
                            "{pad}printf(\"%s\", ({code}) ? \"true\" : \"false\");\n"
                        )),
                        _ => output
                            .push_str(&format!("{pad}printf(\"%lld\", (long long)({code}));\n")),
                    }
                }
                output.push_str(&format!("{pad}printf(\"\\n\");\n"));
            }
            Statement::Expression(expression) => output.push_str(&format!(
                "{pad}{};\n",
                emit_expression(expression, symbols, functions, structs, enums),
            )),
            Statement::Return(value) => {
                if let Some(value) = value {
                    output.push_str(&format!(
                        "{pad}return {};\n",
                        emit_expression(value, symbols, functions, structs, enums),
                    ));
                } else {
                    output.push_str(&format!("{pad}return;\n"));
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                output.push_str(&format!(
                    "{pad}if {} {{\n",
                    emit_condition(condition, symbols, functions, structs, enums),
                ));
                let mut then_symbols = symbols.clone();
                emit_statements(
                    then_branch,
                    &mut then_symbols,
                    functions,
                    structs,
                    enums,
                    indent + 1,
                    output,
                );
                output.push_str(&format!("{pad}}}"));
                if let Some(else_branch) = else_branch {
                    output.push_str(" else {\n");
                    let mut else_symbols = symbols.clone();
                    emit_statements(
                        else_branch,
                        &mut else_symbols,
                        functions,
                        structs,
                        enums,
                        indent + 1,
                        output,
                    );
                    output.push_str(&format!("{pad}}}"));
                }
                output.push('\n');
            }
            Statement::While { condition, body } => {
                output.push_str(&format!(
                    "{pad}while {} {{\n",
                    emit_condition(condition, symbols, functions, structs, enums)
                ));
                let mut nested = symbols.clone();
                emit_statements(
                    body,
                    &mut nested,
                    functions,
                    structs,
                    enums,
                    indent + 1,
                    output,
                );
                output.push_str(&format!("{pad}}}\n"));
            }
            Statement::RepeatUntil { body, condition } => {
                output.push_str(&format!("{pad}do {{\n"));
                let mut nested = symbols.clone();
                emit_statements(
                    body,
                    &mut nested,
                    functions,
                    structs,
                    enums,
                    indent + 1,
                    output,
                );
                output.push_str(&format!(
                    "{pad}}} while (!{});\n",
                    emit_condition(condition, symbols, functions, structs, enums)
                ));
            }
            Statement::ForRange {
                name,
                start,
                end,
                body,
            } => {
                output.push_str(&format!(
                    "{pad}for (int64_t {name} = {}; {name} < {}; ++{name}) {{\n",
                    emit_expression(start, symbols, functions, structs, enums),
                    emit_expression(end, symbols, functions, structs, enums)
                ));
                let mut nested = symbols.clone();
                nested.insert(
                    name.clone(),
                    Symbol {
                        ty: VariableType::Inferred,
                        mutability: Mutability::Constant,
                    },
                );
                emit_statements(
                    body,
                    &mut nested,
                    functions,
                    structs,
                    enums,
                    indent + 1,
                    output,
                );
                output.push_str(&format!("{pad}}}\n"));
            }
            Statement::ForEach {
                name,
                collection,
                body,
            } => {
                if let Expression::Variable(array) = collection {
                    let length = array_length(array, symbols);
                    let iterator = format!("{name}_index");
                    output.push_str(&format!("{pad}for (size_t {iterator} = 0; {iterator} < {length}; ++{iterator}) {{\n{pad}    {} {name} = {array}[{iterator}];\n", c_type(&inferred_type(&Expression::Index { array: Box::new(collection.clone()), index: Box::new(Expression::Literal(Literal::Integer(0))) }, symbols, functions, structs, enums))));
                    let mut nested = symbols.clone();
                    nested.insert(
                        name.clone(),
                        Symbol {
                            ty: inferred_type(
                                &Expression::Index {
                                    array: Box::new(collection.clone()),
                                    index: Box::new(Expression::Literal(Literal::Integer(0))),
                                },
                                symbols,
                                functions,
                                structs,
                                enums,
                            ),
                            mutability: Mutability::Constant,
                        },
                    );
                    emit_statements(
                        body,
                        &mut nested,
                        functions,
                        structs,
                        enums,
                        indent + 1,
                        output,
                    );
                    output.push_str(&format!("{pad}}}\n"));
                }
            }
            Statement::Switch { expression, cases } => {
                let enum_name = cases
                    .iter()
                    .find_map(|case| case.enum_name.as_deref())
                    .unwrap_or("");
                output.push_str(&format!(
                    "{pad}switch (({}).tag) {{\n",
                    emit_expression(expression, symbols, functions, structs, enums)
                ));
                for case in cases {
                    if let (Some(name), Some(variant)) = (&case.enum_name, &case.variant) {
                        if let Some(tag) =
                            enums
                                .iter()
                                .find(|item| item.name == *name)
                                .and_then(|item| {
                                    item.variants
                                        .iter()
                                        .position(|variant_def| variant_def.name == *variant)
                                })
                        {
                            output.push_str(&format!("{pad}case {tag}: {{\n"));
                            let mut nested = symbols.clone();
                            if let Some(variant_def) = enums
                                .iter()
                                .find(|item| item.name == *name)
                                .and_then(|item| {
                                    item.variants.iter().find(|item| item.name == *variant)
                                })
                            {
                                for (index, binding) in case.bindings.iter().enumerate() {
                                    if let Some(ty) = variant_def.payload.get(index) {
                                        output.push_str(&format!(
                                            "{}{} {} = ({}).data.v{}.f{};\n",
                                            "    ".repeat(indent + 2),
                                            c_type(ty),
                                            binding,
                                            emit_expression(
                                                expression, symbols, functions, structs, enums
                                            ),
                                            tag,
                                            index
                                        ));
                                        nested.insert(
                                            binding.clone(),
                                            Symbol {
                                                ty: ty.clone(),
                                                mutability: Mutability::Constant,
                                            },
                                        );
                                    }
                                }
                            }
                            emit_statements(
                                &case.body,
                                &mut nested,
                                functions,
                                structs,
                                enums,
                                indent + 2,
                                output,
                            );
                            output.push_str(&format!(
                                "{}break;\n{pad}}}\n",
                                "    ".repeat(indent + 2)
                            ));
                        }
                    } else {
                        output.push_str(&format!("{pad}default: {{\n"));
                        let mut nested = symbols.clone();
                        emit_statements(
                            &case.body,
                            &mut nested,
                            functions,
                            structs,
                            enums,
                            indent + 2,
                            output,
                        );
                        output.push_str(&format!("{}break;\n{pad}}}\n", "    ".repeat(indent + 2)));
                    }
                }
                let _ = enum_name;
                output.push_str(&format!("{pad}}}\n"));
            }
            Statement::Break => output.push_str(&format!("{pad}break;\n")),
            Statement::Continue => output.push_str(&format!("{pad}continue;\n")),
            Statement::Chunk { body, .. } => {
                output.push_str(&format!("{pad}{{\n"));
                let mut child_symbols = symbols.clone();
                emit_statements(
                    body,
                    &mut child_symbols,
                    functions,
                    structs,
                    enums,
                    indent + 1,
                    output,
                );
                output.push_str(&format!("{pad}}}\n"));
            }
        }
    }
}

fn emit_expression(
    expression: &Expression,
    symbols: &HashMap<String, Symbol>,
    functions: &[Function],
    structs: &[StructDefinition],
    enums: &[EnumDefinition],
) -> String {
    match expression {
        Expression::Literal(Literal::Integer(value)) => value.to_string(),
        Expression::Literal(Literal::Float(value)) => value.to_string(),
        Expression::Literal(Literal::String(value)) => format!("{value:?}"),
        Expression::Literal(Literal::Bool(value)) => value.to_string(),
        Expression::Literal(Literal::Array(_)) => "{0}".to_string(),
        Expression::Literal(Literal::Struct { name, .. }) => format!("({name}){{0}}"),
        Expression::Literal(Literal::Enum { name, variant, .. }) => {
            let tag = enums
                .iter()
                .find(|item| item.name == *name)
                .and_then(|item| item.variants.iter().position(|item| item.name == *variant))
                .unwrap_or(0);
            format!("({name}){{.tag = {tag}}}")
        }
        Expression::Variable(name) => name.clone(),
        Expression::Array(values) => format!(
            "{{ {} }}",
            values
                .iter()
                .map(|value| emit_expression(value, symbols, functions, structs, enums))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::Index { array, index } => {
            let target = emit_expression(array, symbols, functions, structs, enums);
            let length = match array.as_ref() {
                Expression::Variable(name) => array_length(name, symbols),
                _ => "0".into(),
            };
            format!(
                "LDC_INDEX({target}, {}, {length})",
                emit_expression(index, symbols, functions, structs, enums)
            )
        }
        Expression::StructLiteral { name, fields } => format!(
            "({name}){{ {} }}",
            fields
                .iter()
                .map(|(field, value)| format!(
                    ".{field} = {}",
                    emit_expression(value, symbols, functions, structs, enums)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::DefaultStruct(name) => format!("({name}){{0}}"),
        Expression::EnumVariant {
            name,
            variant,
            payload,
        } => {
            let tag = enums
                .iter()
                .find(|item| item.name == *name)
                .and_then(|item| item.variants.iter().position(|item| item.name == *variant))
                .unwrap_or(0);
            let values = payload
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    format!(
                        ".data.v{tag}.f{index} = {}",
                        emit_expression(value, symbols, functions, structs, enums)
                    )
                })
                .collect::<Vec<_>>();
            format!("({name}){{.tag = {tag}, {}}}", values.join(", "))
        }
        Expression::Field { value, name } => format!(
            "({}).{name}",
            emit_expression(value, symbols, functions, structs, enums)
        ),
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
                    symbol
                        .map(|s| sign_label(&s.ty))
                        .unwrap_or("not_applicable")
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
        Expression::Unary {
            operator,
            expression,
        } => format!(
            "({}{})",
            match operator {
                UnaryOperator::Not => "!",
                UnaryOperator::Negate => "-",
                UnaryOperator::BitNot => "~",
            },
            emit_expression(expression, symbols, functions, structs, enums),
        ),
        Expression::Binary {
            left,
            operator: BinaryOperator::Power,
            right,
        } => format!(
            "pow((double)({}), (double)({}))",
            emit_expression(left, symbols, functions, structs, enums),
            emit_expression(right, symbols, functions, structs, enums),
        ),
        Expression::Binary {
            left,
            operator,
            right,
        } if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual) => {
            let (enum_value, enum_name, variant) = match (left.as_ref(), right.as_ref()) {
                (
                    Expression::EnumVariant {
                        name,
                        variant,
                        payload,
                        ..
                    },
                    other,
                ) if payload.is_empty() => (other, name, variant),
                (
                    other,
                    Expression::EnumVariant {
                        name,
                        variant,
                        payload,
                        ..
                    },
                ) if payload.is_empty() => (other, name, variant),
                _ => {
                    return format!(
                        "({} {} {})",
                        emit_expression(left, symbols, functions, structs, enums),
                        operator_symbol(*operator),
                        emit_expression(right, symbols, functions, structs, enums)
                    );
                }
            };
            let tag = enums
                .iter()
                .find(|definition| definition.name == *enum_name)
                .and_then(|definition| {
                    definition
                        .variants
                        .iter()
                        .position(|item| item.name == *variant)
                })
                .unwrap_or(0);
            format!(
                "(({}).tag {} {})",
                emit_expression(enum_value, symbols, functions, structs, enums),
                operator_symbol(*operator),
                tag
            )
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => format!(
            "({} {} {})",
            emit_expression(left, symbols, functions, structs, enums),
            operator_symbol(*operator),
            emit_expression(right, symbols, functions, structs, enums),
        ),
        Expression::Call { name, arguments } => format!(
            "{}",
            if name == "length" {
                arguments
                    .first()
                    .and_then(|arg| {
                        if let Expression::Variable(variable) = arg {
                            match symbols.get(variable).map(|symbol| &symbol.ty) {
                                Some(VariableType::Array { dynamic: true, .. }) => {
                                    Some(format!("{}_len", variable))
                                }
                                Some(VariableType::Array {
                                    capacity,
                                    dynamic: false,
                                    ..
                                }) => Some(capacity.to_string()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "0".into())
            } else if name == "push" && arguments.len() == 2 {
                if let Expression::Variable(variable) = &arguments[0] {
                    let capacity = match symbols.get(variable).map(|s| &s.ty) {
                        Some(VariableType::Array { capacity, .. }) => *capacity,
                        _ => 0,
                    };
                    let dynamic = matches!(
                        symbols.get(variable).map(|symbol| &symbol.ty),
                        Some(VariableType::Array { dynamic: true, .. })
                    );
                    format!(
                        "LDC_PUSH({variable}, {variable}_len, {capacity}, {}, {})",
                        dynamic,
                        emit_expression(&arguments[1], symbols, functions, structs, enums)
                    )
                } else {
                    "((void)0)".into()
                }
            } else if name == "pop" && arguments.len() == 1 {
                if let Expression::Variable(variable) = &arguments[0] {
                    format!("LDC_POP({variable}, {variable}_len)")
                } else {
                    "0".into()
                }
            } else {
                format!(
                    "{}({})",
                    name,
                    arguments
                        .iter()
                        .map(|argument| emit_expression(
                            argument, symbols, functions, structs, enums
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        ),
        Expression::If {
            condition,
            then_value,
            else_value,
        } => format!(
            "({} ? {} : {})",
            emit_expression(condition, symbols, functions, structs, enums),
            emit_expression(then_value, symbols, functions, structs, enums),
            emit_expression(else_value, symbols, functions, structs, enums),
        ),
    }
}

fn emit_condition(
    expression: &Expression,
    symbols: &HashMap<String, Symbol>,
    functions: &[Function],
    structs: &[StructDefinition],
    enums: &[EnumDefinition],
) -> String {
    let code = emit_expression(expression, symbols, functions, structs, enums);
    if matches!(
        expression,
        Expression::Binary { .. } | Expression::Unary { .. } | Expression::If { .. }
    ) {
        code
    } else {
        format!("({code})")
    }
}

fn array_length(name: &str, symbols: &HashMap<String, Symbol>) -> String {
    match symbols.get(name).map(|symbol| &symbol.ty) {
        Some(VariableType::Array { .. }) => format!("{name}_len"),
        _ => "0".to_string(),
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
        VariableType::Inferred
        | VariableType::Integer {
            bits: 32,
            unsigned: false,
        } => "32bit",
        VariableType::Integer {
            bits: 8,
            unsigned: false,
        } => "8bit",
        VariableType::Integer {
            bits: 16,
            unsigned: false,
        } => "16bit",
        VariableType::Integer {
            bits: 8,
            unsigned: true,
        } => "unsigned.8bit",
        VariableType::Integer {
            bits: 16,
            unsigned: true,
        } => "unsigned.16bit",
        VariableType::Integer {
            bits: 32,
            unsigned: true,
        } => "unsigned.32bit",
        VariableType::Float { .. } => "32bit.float",
        VariableType::String { .. } => "16bit.string",
        VariableType::Bool => "bool",
        VariableType::Array { .. } => "array",
        VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_) => "composite",
        VariableType::Integer { .. } => "32bit",
    }
}

fn sign_label(ty: &VariableType) -> &'static str {
    match ty {
        VariableType::Integer { unsigned: true, .. } => "unsigned",
        VariableType::Integer { .. } | VariableType::Inferred | VariableType::Float { .. } => {
            "signed"
        }
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
        VariableType::Array { .. } => 0,
        VariableType::Named(_) | VariableType::Struct(_) | VariableType::Enum(_) => 0,
    }
}
