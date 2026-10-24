use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub environment: Vec<MetadataEntry>,
    pub capabilities: Vec<MetadataEntry>,
    pub identity: Identity,
    pub structs: Vec<StructDefinition>,
    pub enums: Vec<EnumDefinition>,
    pub functions: Vec<Function>,
    pub main: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub mutability: Mutability,
    pub ty: VariableType,
}

#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub name: String,
    pub variants: Vec<EnumVariantDefinition>,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDefinition {
    pub name: String,
    pub payload: Vec<VariableType>,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub enum_name: Option<String>,
    pub variant: Option<String>,
    pub bindings: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<VariableType>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: VariableType,
}

#[derive(Debug, Clone)]
pub struct MetadataEntry {
    pub key: String,
    pub value: MetadataValue,
}

#[derive(Debug, Clone)]
pub enum MetadataValue {
    Literal(Literal),
    Identifier(String),
    Array(Vec<MetadataValue>),
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub name: String,
    pub variables: Vec<VariableDeclaration>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub enum IdentityItem {
    Variable(VariableDeclaration),
    Function(Function),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Variable(VariableDeclaration),
    Assignment {
        name: String,
        value: Expression,
    },
    IndexAssignment {
        name: String,
        index: Expression,
        value: Expression,
    },
    FieldAssignment {
        target: Expression,
        value: Expression,
    },
    Display(Vec<Expression>),
    Expression(Expression),
    Return(Option<Expression>),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    RepeatUntil {
        body: Vec<Statement>,
        condition: Expression,
    },
    ForRange {
        name: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
    },
    ForEach {
        name: String,
        collection: Expression,
        body: Vec<Statement>,
    },
    Switch {
        expression: Expression,
        cases: Vec<SwitchCase>,
    },
    Break,
    Continue,
    Chunk {
        name: String,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub name: String,
    pub mutability: Mutability,
    pub ty: VariableType,
    pub value: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Mutable,
    Constant,
}

#[derive(Debug, Clone)]
pub enum VariableType {
    Inferred,
    Integer {
        bits: u8,
        unsigned: bool,
    },
    Float {
        bits: u8,
    },
    String {
        bits: u8,
    },
    Bool,
    Array {
        element: Box<VariableType>,
        capacity: usize,
        dynamic: bool,
    },
    Struct(String),
    Enum(String),
    Named(String),
}

#[derive(Debug, Clone)]
pub enum ScalarSuffix {
    Float,
    String,
    Bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Vec<Literal>),
    Struct {
        name: String,
        fields: Vec<(String, Literal, Mutability)>,
    },
    Enum {
        name: String,
        variant: String,
        payload: Vec<Literal>,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    Introspection {
        kind: IntrospectionKind,
        argument: Box<Expression>,
    },
    Unary {
        operator: UnaryOperator,
        expression: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Call {
        name: String,
        arguments: Vec<Expression>,
    },
    If {
        condition: Box<Expression>,
        then_value: Box<Expression>,
        else_value: Box<Expression>,
    },
    Array(Vec<Expression>),
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    EnumVariant {
        name: String,
        variant: String,
        payload: Vec<Expression>,
    },
    DefaultStruct(String),
    Field {
        value: Box<Expression>,
        name: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum IntrospectionKind {
    Type,
    Sign,
    BitSize,
    IsMut,
    Identity,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Not,
    Negate,
    BitNot,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    LogicalAnd,
    LogicalOr,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Clone)]
struct RuntimeVariable {
    mutability: Mutability,
    ty: VariableType,
    value: Literal,
}

#[derive(Clone)]
struct Scope {
    frames: Vec<HashMap<String, RuntimeVariable>>,
}

struct TypeRegistry<'a> {
    structs: &'a [StructDefinition],
    enums: &'a [EnumDefinition],
}

enum ExecutionFlow {
    Continue,
    Return(Option<Literal>),
    Break,
    LoopContinue,
}

impl Scope {
    fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
    }

    fn declare(&mut self, declaration: &VariableDeclaration, value: Literal) -> Result<(), String> {
        let value = normalize_array_value(&declaration.ty, value)?;
        declaration.validate_value(&value)?;
        let ty = match (&declaration.ty, &value) {
            (VariableType::Inferred, Literal::Array(values)) => VariableType::Array {
                element: Box::new(
                    values
                        .first()
                        .map(infer_literal_type)
                        .unwrap_or(VariableType::Inferred),
                ),
                capacity: values.len(),
                dynamic: false,
            },
            _ => declaration.ty.clone(),
        };
        let frame = self
            .frames
            .last_mut()
            .expect("uno scope ha sempre un frame");
        if frame.contains_key(&declaration.name) {
            return Err(format!(
                "la variabile '{}' e' gia' stata dichiarata in questo scope",
                declaration.name
            ));
        }
        frame.insert(
            declaration.name.clone(),
            RuntimeVariable {
                mutability: declaration.mutability,
                ty,
                value,
            },
        );
        Ok(())
    }

    fn get(&self, name: &str) -> Option<&RuntimeVariable> {
        self.frames.iter().rev().find_map(|frame| frame.get(name))
    }

    fn assign(&mut self, name: &str, value: Literal) -> Result<(), String> {
        for frame in self.frames.iter_mut().rev() {
            if let Some(variable) = frame.get_mut(name) {
                variable.value = value;
                return Ok(());
            }
        }
        Err(format!("la variabile '{name}' non e' stata dichiarata"))
    }
}

fn normalize_array_value(ty: &VariableType, value: Literal) -> Result<Literal, String> {
    let VariableType::Array {
        element,
        capacity,
        dynamic: false,
    } = ty
    else {
        return Ok(value);
    };
    let Literal::Array(values) = &value else {
        if matches!(element.as_ref(), VariableType::Named(name) if matches!(&value, Literal::Struct { name: value_name, .. } if name == value_name))
        {
            return Ok(Literal::Array(vec![value; *capacity]));
        }
        if is_zero_literal(&value) {
            return Ok(Literal::Array(
                (0..*capacity).map(|_| zero_literal(element)).collect(),
            ));
        }
        return Ok(value);
    };
    if values.is_empty() {
        return Ok(Literal::Array(
            (0..*capacity).map(|_| zero_literal(element)).collect(),
        ));
    }
    Ok(value)
}

fn is_zero_literal(value: &Literal) -> bool {
    match value {
        Literal::Integer(value) => *value == 0,
        Literal::Float(value) => *value == 0.0,
        Literal::String(value) => value.is_empty(),
        Literal::Bool(value) => !value,
        _ => false,
    }
}

fn zero_literal(ty: &VariableType) -> Literal {
    match ty {
        VariableType::Float { .. } => Literal::Float(0.0),
        VariableType::String { .. } => Literal::String(String::new()),
        VariableType::Bool => Literal::Bool(false),
        _ => Literal::Integer(0),
    }
}

fn infer_literal_type(value: &Literal) -> VariableType {
    match value {
        Literal::Integer(_) => VariableType::Integer {
            bits: 32,
            unsigned: false,
        },
        Literal::Float(_) => VariableType::Float { bits: 32 },
        Literal::String(_) => VariableType::String { bits: 16 },
        Literal::Bool(_) => VariableType::Bool,
        Literal::Array(values) => VariableType::Array {
            element: Box::new(
                values
                    .first()
                    .map(infer_literal_type)
                    .unwrap_or(VariableType::Inferred),
            ),
            capacity: values.len(),
            dynamic: false,
        },
        Literal::Struct { name, .. } | Literal::Enum { name, .. } => {
            VariableType::Named(name.clone())
        }
    }
}

impl Program {
    pub fn run(&self) -> Result<Vec<String>, String> {
        let types = TypeRegistry {
            structs: &self.structs,
            enums: &self.enums,
        };
        let mut scope = Scope::new();
        let mut output = Vec::new();

        for declaration in &self.identity.variables {
            let value =
                declaration
                    .value
                    .evaluate(&mut scope, &self.functions, &types, &mut output)?;
            scope.declare(declaration, value)?;
        }

        // Le variabili top-level di fn main hanno il loro scope persistente.
        scope.push_frame();
        match execute_statements(&self.main, &mut scope, &mut output, &self.functions, &types)? {
            ExecutionFlow::Return(_) => {
                return Err("return puo' essere usato solo dentro una funzione".to_string());
            }
            ExecutionFlow::Break | ExecutionFlow::LoopContinue => {
                return Err("break/continue fuori da un ciclo".to_string());
            }
            ExecutionFlow::Continue => {}
        }

        Ok(output)
    }
}

fn execute_statements(
    statements: &[Statement],
    scope: &mut Scope,
    output: &mut Vec<String>,
    functions: &[Function],
    types: &TypeRegistry<'_>,
) -> Result<ExecutionFlow, String> {
    for statement in statements {
        match statement {
            Statement::Variable(declaration) => {
                let value = declaration
                    .value
                    .evaluate(scope, functions, types, output)?;
                scope.declare(declaration, value)?;
            }
            Statement::Assignment { name, value } => {
                let variable = scope
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("la variabile '{name}' non e' stata dichiarata"))?;
                if variable.mutability == Mutability::Constant {
                    return Err(format!("non puoi assegnare a '{name}': e' una let.const"));
                }
                let value = value.evaluate(scope, functions, types, output)?;
                variable.ty.validate_value_named(name, &value)?;
                scope.assign(name, value)?;
            }
            Statement::IndexAssignment { name, index, value } => {
                let current = scope
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("la variabile '{name}' non e' stata dichiarata"))?;
                if current.mutability == Mutability::Constant {
                    return Err(format!("non puoi modificare '{name}': e' una let.const"));
                }
                let index = index
                    .evaluate(scope, functions, types, output)?
                    .as_integer("indice array")?;
                let index =
                    usize::try_from(index).map_err(|_| "indice array negativo".to_string())?;
                let value = value.evaluate(scope, functions, types, output)?;
                let mut array = match current.value {
                    Literal::Array(values) => values,
                    _ => return Err(format!("'{name}' non e' un array")),
                };
                let slot = array.get_mut(index).ok_or_else(|| {
                    format!("indice {index} fuori dai limiti dell'array '{name}'")
                })?;
                match &current.ty {
                    VariableType::Array { element, .. } => {
                        element.validate_value_named(name, &value)?
                    }
                    _ => return Err(format!("'{name}' non e' un array")),
                }
                *slot = value;
                scope.assign(name, Literal::Array(array))?;
            }
            Statement::FieldAssignment { target, value } => {
                assign_field(target, value, scope, functions, types, output)?;
            }
            Statement::Display(expressions) => {
                let values = expressions
                    .iter()
                    .map(|expression| {
                        expression
                            .evaluate(scope, functions, types, output)
                            .map(|value| value.display())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                output.push(values.join(" "));
            }
            Statement::Expression(expression) => {
                if let Expression::Call { name, arguments } = expression {
                    invoke_function(name, arguments, scope, output, functions, types)?;
                } else {
                    return Err(
                        "come istruzione e' ammessa solo una chiamata di funzione".to_string()
                    );
                }
            }
            Statement::Return(expression) => {
                let value = expression
                    .as_ref()
                    .map(|expression| expression.evaluate(scope, functions, types, output))
                    .transpose()?;
                return Ok(ExecutionFlow::Return(value));
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = condition
                    .evaluate(scope, functions, types, output)?
                    .as_bool("la condizione di if")?;
                let branch = if condition {
                    Some(then_branch)
                } else {
                    else_branch.as_ref()
                };
                if let Some(branch) = branch {
                    // Un blocco if ha scope lessicale proprio; le assegnazioni alle
                    // variabili esterne restano invece visibili dopo il blocco.
                    scope.push_frame();
                    let result = execute_statements(branch, scope, output, functions, types);
                    scope.pop_frame();
                    match result? {
                        ExecutionFlow::Continue => {}
                        flow => return Ok(flow),
                    }
                }
            }
            Statement::While { condition, body } => loop {
                if !condition
                    .evaluate(scope, functions, types, output)?
                    .as_bool("la condizione di while")?
                {
                    break;
                }
                scope.push_frame();
                let result = execute_statements(body, scope, output, functions, types);
                scope.pop_frame();
                match result? {
                    ExecutionFlow::Continue | ExecutionFlow::LoopContinue => continue,
                    ExecutionFlow::Break => break,
                    flow @ ExecutionFlow::Return(_) => return Ok(flow),
                }
            },
            Statement::RepeatUntil { body, condition } => loop {
                scope.push_frame();
                let result = execute_statements(body, scope, output, functions, types);
                scope.pop_frame();
                match result? {
                    ExecutionFlow::Break => break,
                    ExecutionFlow::Continue | ExecutionFlow::LoopContinue => {}
                    flow @ ExecutionFlow::Return(_) => return Ok(flow),
                }
                if condition
                    .evaluate(scope, functions, types, output)?
                    .as_bool("la condizione di until")?
                {
                    break;
                }
            },
            Statement::ForRange {
                name,
                start,
                end,
                body,
            } => {
                let mut index = start
                    .evaluate(scope, functions, types, output)?
                    .as_integer("inizio range")?;
                let end = end
                    .evaluate(scope, functions, types, output)?
                    .as_integer("fine range")?;
                while index < end {
                    scope.push_frame();
                    scope.declare(
                        &VariableDeclaration {
                            name: name.clone(),
                            mutability: Mutability::Constant,
                            ty: VariableType::Inferred,
                            value: Expression::Literal(Literal::Integer(index)),
                        },
                        Literal::Integer(index),
                    )?;
                    let result = execute_statements(body, scope, output, functions, types);
                    scope.pop_frame();
                    match result? {
                        ExecutionFlow::Break => break,
                        ExecutionFlow::Continue | ExecutionFlow::LoopContinue => {}
                        flow @ ExecutionFlow::Return(_) => return Ok(flow),
                    }
                    index = index
                        .checked_add(1)
                        .ok_or_else(|| "overflow nel contatore di for".to_string())?;
                }
            }
            Statement::ForEach {
                name,
                collection,
                body,
            } => {
                let values = match collection.evaluate(scope, functions, types, output)? {
                    Literal::Array(values) => values,
                    _ => return Err("for-in richiede un array".to_string()),
                };
                for value in values {
                    scope.push_frame();
                    scope.declare(
                        &VariableDeclaration {
                            name: name.clone(),
                            mutability: Mutability::Constant,
                            ty: infer_literal_type(&value),
                            value: Expression::Literal(value.clone()),
                        },
                        value,
                    )?;
                    let result = execute_statements(body, scope, output, functions, types);
                    scope.pop_frame();
                    match result? {
                        ExecutionFlow::Break => break,
                        ExecutionFlow::Continue | ExecutionFlow::LoopContinue => {}
                        flow @ ExecutionFlow::Return(_) => return Ok(flow),
                    }
                }
            }
            Statement::Switch { expression, cases } => {
                let value = expression.evaluate(scope, functions, types, output)?;
                let (enum_name, variant, payload) = match value {
                    Literal::Enum {
                        name,
                        variant,
                        payload,
                    } => (name, variant, payload),
                    _ => return Err("switch richiede un valore enum".to_string()),
                };
                let selected = cases
                    .iter()
                    .find(|case| {
                        case.enum_name.as_deref() == Some(&enum_name)
                            && case.variant.as_deref() == Some(&variant)
                    })
                    .or_else(|| cases.iter().find(|case| case.enum_name.is_none()))
                    .ok_or_else(|| "switch senza case corrispondente o default".to_string())?;
                if !selected.bindings.is_empty() && selected.bindings.len() != payload.len() {
                    return Err(format!(
                        "pattern {enum_name}::{variant} ha {} binding per {} valori",
                        selected.bindings.len(),
                        payload.len()
                    ));
                }
                scope.push_frame();
                for (name, value) in selected.bindings.iter().zip(payload) {
                    let ty = types
                        .enums
                        .iter()
                        .find(|item| item.name == enum_name)
                        .and_then(|item| item.variants.iter().find(|v| v.name == variant))
                        .and_then(|v| {
                            v.payload.get(
                                selected
                                    .bindings
                                    .iter()
                                    .position(|binding| binding == name)
                                    .unwrap_or(0),
                            )
                        })
                        .cloned()
                        .unwrap_or_else(|| infer_literal_type(&value));
                    scope.declare(
                        &VariableDeclaration {
                            name: name.clone(),
                            mutability: Mutability::Constant,
                            ty,
                            value: Expression::Literal(value.clone()),
                        },
                        value,
                    )?;
                }
                let result = execute_statements(&selected.body, scope, output, functions, types);
                scope.pop_frame();
                match result? {
                    ExecutionFlow::Continue | ExecutionFlow::Break => {}
                    flow => return Ok(flow),
                }
            }
            Statement::Break => return Ok(ExecutionFlow::Break),
            Statement::Continue => return Ok(ExecutionFlow::LoopContinue),
            Statement::Chunk { name: _, body } => {
                // Il figlio eredita una copia dell'ambiente: puo' leggere e modificare
                // una let.mut ereditata, ma la modifica viene scartata alla chiusura.
                let mut child_scope = scope.clone();
                child_scope.push_frame();
                match execute_statements(body, &mut child_scope, output, functions, types)? {
                    ExecutionFlow::Continue => {}
                    flow => return Ok(flow),
                }
            }
        }
    }
    Ok(ExecutionFlow::Continue)
}

enum PathPart {
    Field(String),
    Index(usize),
}

fn assign_field(
    target: &Expression,
    expression: &Expression,
    scope: &mut Scope,
    functions: &[Function],
    types: &TypeRegistry<'_>,
    output: &mut Vec<String>,
) -> Result<(), String> {
    fn collect(
        target: &Expression,
        scope: &mut Scope,
        functions: &[Function],
        types: &TypeRegistry<'_>,
        output: &mut Vec<String>,
        path: &mut Vec<PathPart>,
    ) -> Result<String, String> {
        match target {
            Expression::Variable(name) => Ok(name.clone()),
            Expression::Field { value, name } => {
                let root = collect(value, scope, functions, types, output, path)?;
                path.push(PathPart::Field(name.clone()));
                Ok(root)
            }
            Expression::Index { array, index } => {
                let root = collect(array, scope, functions, types, output, path)?;
                let index = index
                    .evaluate(scope, functions, types, output)?
                    .as_integer("indice array")?;
                let index =
                    usize::try_from(index).map_err(|_| "indice array negativo".to_string())?;
                path.push(PathPart::Index(index));
                Ok(root)
            }
            _ => Err("assegnazione possibile solo a un campo o elemento nominato".to_string()),
        }
    }
    fn update(
        value: &mut Literal,
        path: &[PathPart],
        replacement: Literal,
        type_registry: &TypeRegistry<'_>,
    ) -> Result<(), String> {
        let Some((part, tail)) = path.split_first() else {
            *value = replacement;
            return Ok(());
        };
        match part {
            PathPart::Index(index) => match value {
                Literal::Array(values) => {
                    let slot = values
                        .get_mut(*index)
                        .ok_or_else(|| format!("indice {index} fuori dai limiti dell'array"))?;
                    update(slot, tail, replacement, type_registry)
                }
                _ => Err("l'indice richiede un array".to_string()),
            },
            PathPart::Field(name) => match value {
                Literal::Struct {
                    name: type_name,
                    fields,
                } => {
                    let field = fields
                        .iter_mut()
                        .find(|(field_name, _, _)| field_name == name)
                        .ok_or_else(|| format!("campo '{name}' non trovato in '{type_name}'"))?;
                    if field.2 == Mutability::Constant {
                        return Err(format!("il campo '{name}' e' const"));
                    }
                    if tail.is_empty() {
                        let definition = type_registry
                            .structs
                            .iter()
                            .find(|definition| definition.name == *type_name)
                            .and_then(|definition| {
                                definition.fields.iter().find(|field| field.name == *name)
                            })
                            .ok_or_else(|| format!("definizione del campo '{name}' non trovata"))?;
                        definition.ty.validate_value_named(name, &replacement)?;
                    }
                    update(&mut field.1, tail, replacement, type_registry)
                }
                _ => Err("l'accesso con '.' richiede una struct".to_string()),
            },
        }
    }
    let replacement = expression.evaluate(scope, functions, types, output)?;
    let mut path = Vec::new();
    let root = collect(target, scope, functions, types, output, &mut path)?;
    let variable = scope
        .get(&root)
        .cloned()
        .ok_or_else(|| format!("la variabile '{root}' non e' stata dichiarata"))?;
    let mut value = variable.value;
    update(&mut value, &path, replacement, types)?;
    scope.assign(&root, value)
}

fn invoke_function(
    name: &str,
    arguments: &[Expression],
    scope: &mut Scope,
    output: &mut Vec<String>,
    functions: &[Function],
    types: &TypeRegistry<'_>,
) -> Result<Option<Literal>, String> {
    if name == "length" || name == "push" || name == "pop" {
        let expected = if name == "push" { 2 } else { 1 };
        if arguments.len() != expected {
            return Err(format!("'{name}' richiede {expected} argomenti"));
        }
        let array_name = match &arguments[0] {
            Expression::Variable(name) => name,
            _ => {
                return Err(format!(
                    "il primo argomento di '{name}' deve essere una variabile array"
                ));
            }
        };
        let variable = scope
            .get(array_name)
            .cloned()
            .ok_or_else(|| format!("la variabile '{array_name}' non e' stata dichiarata"))?;
        let mut items = match variable.value {
            Literal::Array(items) => items,
            _ => return Err(format!("'{array_name}' non e' un array")),
        };
        match name {
            "length" => return Ok(Some(Literal::Integer(items.len() as i64))),
            "push" => {
                if variable.mutability == Mutability::Constant {
                    return Err(format!(
                        "non puoi modificare '{array_name}': e' una let.const"
                    ));
                }
                let (element, capacity, dynamic) = match &variable.ty {
                    VariableType::Array {
                        element,
                        capacity,
                        dynamic,
                    } => (element, capacity, dynamic),
                    _ => return Err(format!("'{array_name}' non e' un array tipizzato")),
                };
                if !dynamic {
                    return Err(format!(
                        "push richiede un array dinamico, '{array_name}' e' a dimensione fissa"
                    ));
                }
                if items.len() >= *capacity {
                    return Err(format!(
                        "capacita' massima raggiunta per l'array '{array_name}'"
                    ));
                }
                let value = arguments[1].evaluate(scope, functions, types, output)?;
                element.validate_value_named(array_name, &value)?;
                items.push(value);
                scope.assign(array_name, Literal::Array(items))?;
                return Ok(None);
            }
            "pop" => {
                if variable.mutability == Mutability::Constant {
                    return Err(format!(
                        "non puoi modificare '{array_name}': e' una let.const"
                    ));
                }
                let value = items
                    .pop()
                    .ok_or_else(|| format!("pop su array vuoto '{array_name}'"))?;
                scope.assign(array_name, Literal::Array(items))?;
                return Ok(Some(value));
            }
            _ => unreachable!(),
        }
    }
    let function = functions
        .iter()
        .find(|function| function.name == name)
        .cloned()
        .ok_or_else(|| format!("la funzione '{name}' non e' stata dichiarata"))?;
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "la funzione '{name}' richiede {} argomenti, ricevuti {}",
            function.parameters.len(),
            arguments.len()
        ));
    }

    let values = arguments
        .iter()
        .map(|argument| argument.evaluate(scope, functions, types, output))
        .collect::<Result<Vec<_>, _>>()?;

    scope.push_frame();
    let setup_result = function
        .parameters
        .iter()
        .zip(values)
        .try_for_each(|(parameter, value)| {
            parameter.ty.validate_value_named(&parameter.name, &value)?;
            scope.declare(
                &VariableDeclaration {
                    name: parameter.name.clone(),
                    mutability: Mutability::Constant,
                    ty: parameter.ty.clone(),
                    value: Expression::Literal(value.clone()),
                },
                value,
            )
        });
    if let Err(error) = setup_result {
        scope.pop_frame();
        return Err(error);
    }

    let execution = execute_statements(&function.body, scope, output, functions, types);
    scope.pop_frame();
    let returned = match execution? {
        ExecutionFlow::Continue => None,
        ExecutionFlow::Return(value) => Some(value),
        ExecutionFlow::Break | ExecutionFlow::LoopContinue => {
            return Err("break/continue fuori da un ciclo".to_string());
        }
    };

    match (function.return_type, returned) {
        (Some(expected), Some(Some(value))) => {
            expected.validate_value_named("valore restituito", &value)?;
            Ok(Some(value))
        }
        (Some(_), _) => Err(format!("la funzione '{name}' non ha restituito un valore")),
        (None, Some(Some(_))) => Err(format!(
            "la funzione '{name}' e' void e non puo' restituire un valore"
        )),
        (None, Some(None) | None) => Ok(None),
    }
}

impl VariableDeclaration {
    fn validate_value(&self, value: &Literal) -> Result<(), String> {
        self.ty.validate_value_named(&self.name, value)
    }
}

impl VariableType {
    fn validate_value_named(&self, name: &str, value: &Literal) -> Result<(), String> {
        match (self, value) {
            (Self::Inferred, Literal::Integer(value)) => {
                if *value < i32::MIN as i64 || *value > i32::MAX as i64 {
                    Err(format!(
                        "il valore {value} non rientra nel tipo i32 dedotto della variabile '{name}'"
                    ))
                } else {
                    Ok(())
                }
            }
            (Self::Inferred, _) => Ok(()),
            (Self::Bool, Literal::Bool(_)) => Ok(()),
            (Self::String { .. }, Literal::String(_)) => Ok(()),
            (Self::Float { .. }, Literal::Float(_)) => Ok(()),
            (
                Self::Array {
                    element,
                    capacity,
                    dynamic,
                },
                Literal::Array(values),
            ) => {
                if (!*dynamic && values.len() != *capacity)
                    || (*dynamic && values.len() > *capacity)
                {
                    return Err(format!(
                        "dimensione non valida per l'array '{name}': {} elementi, capacita' {capacity}",
                        values.len()
                    ));
                }
                for item in values {
                    element.validate_value_named(name, item)?;
                }
                Ok(())
            }
            (Self::Named(expected), Literal::Struct { name, .. } | Literal::Enum { name, .. })
                if expected == name =>
            {
                Ok(())
            }
            (Self::Integer { bits, unsigned }, Literal::Integer(value)) => {
                let (minimum, maximum) = integer_range(*bits, *unsigned);
                if *value < minimum || *value > maximum {
                    return Err(format!(
                        "il valore {value} non rientra nel tipo {}{}bit della variabile '{name}'",
                        if *unsigned { "unsigned." } else { "" },
                        bits
                    ));
                }
                Ok(())
            }
            (expected, actual) => Err(format!(
                "tipo incompatibile per '{name}': atteso {}, trovato {}",
                expected.name(),
                actual.name()
            )),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Inferred => "un valore deducibile",
            Self::Integer { .. } => "intero",
            Self::Float { .. } => "float",
            Self::String { .. } => "stringa",
            Self::Bool => "booleano",
            Self::Array { .. } => "array",
            Self::Named(_) | Self::Struct(_) | Self::Enum(_) => "tipo composto",
        }
    }

    fn type_name(&self, value: &Literal) -> &'static str {
        match self {
            Self::Inferred => match value {
                Literal::Integer(_) => "32bit",
                Literal::Float(_) => "float",
                Literal::String(_) => "string",
                Literal::Bool(_) => "bool",
                Literal::Array(_) => "array",
                Literal::Struct { .. } => "struct",
                Literal::Enum { .. } => "enum",
            },
            Self::Integer {
                bits: 8,
                unsigned: true,
            } => "unsigned.8bit",
            Self::Integer {
                bits: 16,
                unsigned: true,
            } => "unsigned.16bit",
            Self::Integer {
                bits: 32,
                unsigned: true,
            } => "unsigned.32bit",
            Self::Integer {
                bits: 8,
                unsigned: false,
            } => "8bit",
            Self::Integer {
                bits: 16,
                unsigned: false,
            } => "16bit",
            Self::Integer {
                bits: 32,
                unsigned: false,
            } => "32bit",
            Self::Float { bits: 8 } => "8bit.float",
            Self::Float { bits: 16 } => "16bit.float",
            Self::Float { bits: 32 } => "32bit.float",
            Self::String { bits: 8 } => "8bit.string",
            Self::String { bits: 16 } => "16bit.string",
            Self::String { bits: 32 } => "32bit.string",
            Self::Bool => "bool",
            Self::Array { .. } => "array",
            Self::Named(_) | Self::Struct(_) | Self::Enum(_) => "tipo composto",
            _ => unreachable!("la grammatica ammette solo 8, 16 o 32 bit"),
        }
    }

    fn sign(&self, value: &Literal) -> &'static str {
        match self {
            Self::Integer { unsigned: true, .. } => "unsigned",
            Self::Integer {
                unsigned: false, ..
            } => "signed",
            Self::Inferred if matches!(value, Literal::Integer(_)) => "signed",
            _ => "not_applicable",
        }
    }

    fn bit_size(&self, value: &Literal) -> u8 {
        match self {
            Self::Integer { bits, .. } | Self::Float { bits } | Self::String { bits } => *bits,
            Self::Inferred if matches!(value, Literal::Integer(_)) => 32,
            Self::Bool => 1,
            Self::Array { .. } => 0,
            Self::Named(_) | Self::Struct(_) | Self::Enum(_) => 0,
            Self::Inferred => 0,
        }
    }
}

impl Literal {
    fn name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "intero",
            Self::Float(_) => "float",
            Self::String(_) => "stringa",
            Self::Bool(_) => "booleano",
            Self::Array(_) => "array",
            Self::Struct { .. } => "struct",
            Self::Enum { .. } => "enum",
        }
    }

    fn display(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Array(values) => format!("[{} elementi]", values.len()),
            Self::Struct { name, .. } => format!("{name} {{...}}"),
            Self::Enum { name, variant, .. } => format!("{name}::{variant}"),
        }
    }

    fn as_bool(&self, context: &str) -> Result<bool, String> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(format!(
                "{context} deve essere un booleano, trovato {}",
                self.name()
            )),
        }
    }

    fn as_integer(&self, operator: &str) -> Result<i64, String> {
        match self {
            Self::Integer(value) => Ok(*value),
            _ => Err(format!("l'operatore '{operator}' richiede valori interi")),
        }
    }

    fn as_number(&self, operator: &str) -> Result<f64, String> {
        match self {
            Self::Integer(value) => Ok(*value as f64),
            Self::Float(value) => Ok(*value),
            _ => Err(format!("l'operatore '{operator}' richiede valori numerici")),
        }
    }
}

impl Expression {
    fn evaluate(
        &self,
        scope: &mut Scope,
        functions: &[Function],
        types: &TypeRegistry<'_>,
        output: &mut Vec<String>,
    ) -> Result<Literal, String> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Variable(name) => scope
                .get(name)
                .map(|variable| variable.value.clone())
                .ok_or_else(|| format!("la variabile '{name}' non e' stata dichiarata")),
            Self::Introspection { kind, argument } => {
                let name = match argument.as_ref() {
                    Self::Variable(name) => name,
                    _ => {
                        return Err(format!(
                            "{} richiede il nome diretto di una variabile, non un'espressione",
                            kind.name()
                        ));
                    }
                };
                let variable = scope
                    .get(name)
                    .ok_or_else(|| format!("la variabile '{name}' non e' stata dichiarata"))?;
                Ok(kind.describe(variable))
            }
            Self::Unary {
                operator,
                expression,
            } => operator.evaluate(expression.evaluate(scope, functions, types, output)?),
            Self::Binary {
                left,
                operator,
                right,
            } => {
                if matches!(operator, BinaryOperator::LogicalAnd) {
                    let left = left
                        .evaluate(scope, functions, types, output)?
                        .as_bool("il lato sinistro di &&")?;
                    return if left {
                        Ok(Literal::Bool(
                            right
                                .evaluate(scope, functions, types, output)?
                                .as_bool("il lato destro di &&")?,
                        ))
                    } else {
                        Ok(Literal::Bool(false))
                    };
                }
                if matches!(operator, BinaryOperator::LogicalOr) {
                    let left = left
                        .evaluate(scope, functions, types, output)?
                        .as_bool("il lato sinistro di ||")?;
                    return if left {
                        Ok(Literal::Bool(true))
                    } else {
                        Ok(Literal::Bool(
                            right
                                .evaluate(scope, functions, types, output)?
                                .as_bool("il lato destro di ||")?,
                        ))
                    };
                }
                let left_value = left.evaluate(scope, functions, types, output)?;
                let right_value = right.evaluate(scope, functions, types, output)?;
                if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
                    && !matches!(left.as_ref(), Self::EnumVariant { payload, .. } if payload.is_empty())
                    && !matches!(right.as_ref(), Self::EnumVariant { payload, .. } if payload.is_empty())
                    && (matches!(&left_value, Literal::Enum { .. })
                        || matches!(&right_value, Literal::Enum { .. }))
                {
                    return Err(
                        "un enum con payload si confronta con switch, non con ==/!=".to_string()
                    );
                }
                operator.evaluate(left_value, right_value)
            }
            Self::Call { name, arguments } => invoke_function(
                name, arguments, scope, output, functions, types,
            )?
            .ok_or_else(|| format!("la funzione '{name}' e' void e non restituisce un valore")),
            Self::Array(values) => Ok(Literal::Array(
                values
                    .iter()
                    .map(|value| value.evaluate(scope, functions, types, output))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Self::Index { array, index } => {
                let array = array.evaluate(scope, functions, types, output)?;
                let index = index
                    .evaluate(scope, functions, types, output)?
                    .as_integer("indice array")?;
                let index =
                    usize::try_from(index).map_err(|_| "indice array negativo".to_string())?;
                match array {
                    Literal::Array(values) => values
                        .get(index)
                        .cloned()
                        .ok_or_else(|| format!("indice {index} fuori dai limiti dell'array")),
                    _ => Err("l'indicizzazione richiede un array".to_string()),
                }
            }
            Self::StructLiteral { name, fields } => {
                let definition = types
                    .structs
                    .iter()
                    .find(|definition| definition.name == *name)
                    .ok_or_else(|| format!("struct '{name}' non dichiarata"))?;
                if fields.len() != definition.fields.len() {
                    return Err(format!(
                        "inizializzatore di '{name}': attesi {} campi, ricevuti {}",
                        definition.fields.len(),
                        fields.len()
                    ));
                }
                let mut result = Vec::with_capacity(fields.len());
                for field in &definition.fields {
                    let (_, expression) = fields
                        .iter()
                        .find(|(field_name, _)| field_name == &field.name)
                        .ok_or_else(|| format!("campo '{}' mancante in '{name}'", field.name))?;
                    let value = expression.evaluate(scope, functions, types, output)?;
                    field.ty.validate_value_named(&field.name, &value)?;
                    result.push((field.name.clone(), value, field.mutability));
                }
                Ok(Literal::Struct {
                    name: name.clone(),
                    fields: result,
                })
            }
            Self::DefaultStruct(name) => {
                let definition = types
                    .structs
                    .iter()
                    .find(|definition| definition.name == *name)
                    .ok_or_else(|| format!("struct '{name}' non dichiarata"))?;
                let fields = definition
                    .fields
                    .iter()
                    .map(|field| {
                        Ok((
                            field.name.clone(),
                            default_value(&field.ty, types)?,
                            field.mutability,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Literal::Struct {
                    name: name.clone(),
                    fields,
                })
            }
            Self::EnumVariant {
                name,
                variant,
                payload,
            } => {
                let definition = types
                    .enums
                    .iter()
                    .find(|definition| definition.name == *name)
                    .ok_or_else(|| format!("enum '{name}' non dichiarato"))?;
                let variant_def = definition
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .ok_or_else(|| format!("variante '{name}::{variant}' non dichiarata"))?;
                if payload.len() != variant_def.payload.len() {
                    return Err(format!(
                        "variante '{name}::{variant}' richiede {} valori",
                        variant_def.payload.len()
                    ));
                }
                let values = payload
                    .iter()
                    .zip(&variant_def.payload)
                    .map(|(expression, ty)| {
                        let value = expression.evaluate(scope, functions, types, output)?;
                        ty.validate_value_named("payload enum", &value)?;
                        Ok(value)
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Literal::Enum {
                    name: name.clone(),
                    variant: variant.clone(),
                    payload: values,
                })
            }
            Self::Field { value, name } => {
                match value.evaluate(scope, functions, types, output)? {
                    Literal::Struct { fields, .. } => fields
                        .into_iter()
                        .find(|(field, _, _)| field == name)
                        .map(|(_, value, _)| value)
                        .ok_or_else(|| format!("campo '{name}' non trovato")),
                    _ => Err("l'accesso con '.' richiede una struct".to_string()),
                }
            }
            Self::If {
                condition,
                then_value,
                else_value,
            } => {
                if condition
                    .evaluate(scope, functions, types, output)?
                    .as_bool("la condizione di if")?
                {
                    then_value.evaluate(scope, functions, types, output)
                } else {
                    else_value.evaluate(scope, functions, types, output)
                }
            }
        }
    }
}

fn default_value(ty: &VariableType, types: &TypeRegistry<'_>) -> Result<Literal, String> {
    Ok(match ty {
        VariableType::Inferred | VariableType::Integer { .. } => Literal::Integer(0),
        VariableType::Float { .. } => Literal::Float(0.0),
        VariableType::String { .. } => Literal::String(String::new()),
        VariableType::Bool => Literal::Bool(false),
        VariableType::Array {
            element,
            capacity,
            dynamic,
        } => {
            let count = if *dynamic { 0 } else { *capacity };
            Literal::Array(
                (0..count)
                    .map(|_| default_value(element, types))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        VariableType::Named(name) | VariableType::Struct(name) => {
            let definition = types
                .structs
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| format!("tipo '{name}' non inizializzabile con default()"))?;
            Literal::Struct {
                name: name.clone(),
                fields: definition
                    .fields
                    .iter()
                    .map(|field| {
                        Ok((
                            field.name.clone(),
                            default_value(&field.ty, types)?,
                            field.mutability,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            }
        }
        VariableType::Enum(_) => return Err("default() su enum non è supportato".to_string()),
    })
}

impl IntrospectionKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Type => "type_of",
            Self::Sign => "sign_of",
            Self::BitSize => "bit_size_of",
            Self::IsMut => "is_mut",
            Self::Identity => "identity_of",
        }
    }

    fn describe(&self, variable: &RuntimeVariable) -> Literal {
        match self {
            Self::Type => Literal::String(variable.ty.type_name(&variable.value).to_string()),
            Self::Sign => Literal::String(variable.ty.sign(&variable.value).to_string()),
            Self::BitSize => Literal::Integer(variable.ty.bit_size(&variable.value) as i64),
            Self::IsMut => Literal::Bool(variable.mutability == Mutability::Mutable),
            Self::Identity => Literal::String(format!(
                "{{ type: {}, sign: {}, bits: {}, mut: {} }}",
                variable.ty.type_name(&variable.value),
                variable.ty.sign(&variable.value),
                variable.ty.bit_size(&variable.value),
                variable.mutability == Mutability::Mutable
            )),
        }
    }
}

impl UnaryOperator {
    fn evaluate(self, value: Literal) -> Result<Literal, String> {
        match self {
            Self::Not => Ok(Literal::Bool(!value.as_bool("l'operatore '!'")?)),
            Self::Negate => match value {
                Literal::Integer(value) => value
                    .checked_neg()
                    .map(Literal::Integer)
                    .ok_or_else(|| "overflow nell'operatore '-'".to_string()),
                Literal::Float(value) => Ok(Literal::Float(-value)),
                _ => Err("l'operatore '-' richiede un valore numerico".to_string()),
            },
            Self::BitNot => Ok(Literal::Integer(!value.as_integer("~")?)),
        }
    }
}

impl BinaryOperator {
    fn evaluate(self, left: Literal, right: Literal) -> Result<Literal, String> {
        match self {
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Remainder
            | Self::Power => self.evaluate_math(left, right),
            Self::Equal
            | Self::NotEqual
            | Self::Greater
            | Self::Less
            | Self::GreaterEqual
            | Self::LessEqual => self.evaluate_comparison(left, right),
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::ShiftLeft | Self::ShiftRight => {
                self.evaluate_bitwise(left, right)
            }
            Self::LogicalAnd | Self::LogicalOr => {
                unreachable!("gli operatori logici sono short-circuit")
            }
        }
    }

    fn evaluate_math(self, left: Literal, right: Literal) -> Result<Literal, String> {
        if matches!(self, Self::Add) {
            if let (Literal::String(left), Literal::String(right)) = (&left, &right) {
                return Ok(Literal::String(format!("{left}{right}")));
            }
        }

        if let (Literal::Integer(left), Literal::Integer(right)) = (&left, &right) {
            let value = match self {
                Self::Add => left.checked_add(*right),
                Self::Subtract => left.checked_sub(*right),
                Self::Multiply => left.checked_mul(*right),
                Self::Divide => {
                    if *right == 0 {
                        return Err("divisione per zero".to_string());
                    }
                    left.checked_div(*right)
                }
                Self::Remainder => {
                    if *right == 0 {
                        return Err("divisione per zero".to_string());
                    }
                    left.checked_rem(*right)
                }
                Self::Power => {
                    let exponent = u32::try_from(*right)
                        .map_err(|_| "l'esponente intero non puo' essere negativo".to_string())?;
                    left.checked_pow(exponent)
                }
                _ => unreachable!(),
            };
            return value
                .map(Literal::Integer)
                .ok_or_else(|| format!("overflow nell'operatore '{}'", self.symbol()));
        }

        let left = left.as_number(self.symbol())?;
        let right = right.as_number(self.symbol())?;
        let value = match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
            Self::Divide => {
                if right == 0.0 {
                    return Err("divisione per zero".to_string());
                }
                left / right
            }
            Self::Remainder => {
                if right == 0.0 {
                    return Err("divisione per zero".to_string());
                }
                left % right
            }
            Self::Power => left.powf(right),
            _ => unreachable!(),
        };
        if value.is_finite() {
            Ok(Literal::Float(value))
        } else {
            Err(format!("overflow nell'operatore '{}'", self.symbol()))
        }
    }

    fn evaluate_comparison(self, left: Literal, right: Literal) -> Result<Literal, String> {
        let result = match self {
            Self::Equal => values_equal(&left, &right),
            Self::NotEqual => !values_equal(&left, &right),
            Self::Greater | Self::Less | Self::GreaterEqual | Self::LessEqual => {
                let left = left.as_number(self.symbol())?;
                let right = right.as_number(self.symbol())?;
                match self {
                    Self::Greater => left > right,
                    Self::Less => left < right,
                    Self::GreaterEqual => left >= right,
                    Self::LessEqual => left <= right,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        Ok(Literal::Bool(result))
    }

    fn evaluate_bitwise(self, left: Literal, right: Literal) -> Result<Literal, String> {
        let left = left.as_integer(self.symbol())?;
        let right = right.as_integer(self.symbol())?;
        let value = match self {
            Self::BitAnd => left & right,
            Self::BitOr => left | right,
            Self::BitXor => left ^ right,
            Self::ShiftLeft => {
                let shift = u32::try_from(right)
                    .map_err(|_| "lo shift non puo' essere negativo".to_string())?;
                left.checked_shl(shift)
                    .ok_or_else(|| "overflow nello shift a sinistra".to_string())?
            }
            Self::ShiftRight => {
                let shift = u32::try_from(right)
                    .map_err(|_| "lo shift non puo' essere negativo".to_string())?;
                left.checked_shr(shift)
                    .ok_or_else(|| "overflow nello shift a destra".to_string())?
            }
            _ => unreachable!(),
        };
        Ok(Literal::Integer(value))
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Power => "**",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Greater => ">",
            Self::Less => "<",
            Self::GreaterEqual => ">=",
            Self::LessEqual => "<=",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
        }
    }
}

fn values_equal(left: &Literal, right: &Literal) -> bool {
    match (left, right) {
        (Literal::Integer(left), Literal::Float(right)) => *left as f64 == *right,
        (Literal::Float(left), Literal::Integer(right)) => *left == *right as f64,
        _ => left == right,
    }
}

fn integer_range(bits: u8, unsigned: bool) -> (i64, i64) {
    if unsigned {
        (0, (1_i64 << bits) - 1)
    } else {
        let limit = 1_i64 << (bits - 1);
        (-limit, limit - 1)
    }
}
