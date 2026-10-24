use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub environment: Vec<MetadataEntry>,
    pub capabilities: Vec<MetadataEntry>,
    pub identity: Identity,
    pub main: Vec<Statement>,
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
}

#[derive(Debug, Clone)]
pub enum Statement {
    Variable(VariableDeclaration),
    Assignment {
        name: String,
        value: Expression,
    },
    Display(Vec<Expression>),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
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
    Integer { bits: u8, unsigned: bool },
    Float { bits: u8 },
    String { bits: u8 },
    Bool,
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
    If {
        condition: Box<Expression>,
        then_value: Box<Expression>,
        else_value: Box<Expression>,
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
        declaration.validate_value(&value)?;
        let frame = self.frames.last_mut().expect("uno scope ha sempre un frame");
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
                ty: declaration.ty.clone(),
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

impl Program {
    pub fn run(&self) -> Result<Vec<String>, String> {
        let mut scope = Scope::new();
        let mut output = Vec::new();

        for declaration in &self.identity.variables {
            let value = declaration.value.evaluate(&scope)?;
            scope.declare(declaration, value)?;
        }

        // Le variabili top-level di fn main hanno il loro scope persistente.
        scope.push_frame();
        execute_statements(&self.main, &mut scope, &mut output)?;

        Ok(output)
    }
}

fn execute_statements(
    statements: &[Statement],
    scope: &mut Scope,
    output: &mut Vec<String>,
) -> Result<(), String> {
    for statement in statements {
        match statement {
            Statement::Variable(declaration) => {
                let value = declaration.value.evaluate(scope)?;
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
                let value = value.evaluate(scope)?;
                variable.ty.validate_value_named(name, &value)?;
                scope.assign(name, value)?;
            }
            Statement::Display(expressions) => {
                let values = expressions
                    .iter()
                    .map(|expression| expression.evaluate(scope).map(|value| value.display()))
                    .collect::<Result<Vec<_>, _>>()?;
                output.push(values.join(" "));
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = condition.evaluate(scope)?.as_bool("la condizione di if")?;
                let branch = if condition {
                    Some(then_branch)
                } else {
                    else_branch.as_ref()
                };
                if let Some(branch) = branch {
                    // Un blocco if ha scope lessicale proprio; le assegnazioni alle
                    // variabili esterne restano invece visibili dopo il blocco.
                    scope.push_frame();
                    let result = execute_statements(branch, scope, output);
                    scope.pop_frame();
                    result?;
                }
            }
            Statement::Chunk { name: _, body } => {
                // Il figlio eredita una copia dell'ambiente: puo' leggere e modificare
                // una let.mut ereditata, ma la modifica viene scartata alla chiusura.
                let mut child_scope = scope.clone();
                child_scope.push_frame();
                execute_statements(body, &mut child_scope, output)?;
            }
        }
    }
    Ok(())
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
        }
    }

    fn type_name(&self, value: &Literal) -> &'static str {
        match self {
            Self::Inferred => match value {
                Literal::Integer(_) => "32bit",
                Literal::Float(_) => "float",
                Literal::String(_) => "string",
                Literal::Bool(_) => "bool",
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
        }
    }

    fn display(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
        }
    }

    fn as_bool(&self, context: &str) -> Result<bool, String> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(format!("{context} deve essere un booleano, trovato {}", self.name())),
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
    fn evaluate(&self, scope: &Scope) -> Result<Literal, String> {
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
            } => operator.evaluate(expression.evaluate(scope)?),
            Self::Binary {
                left,
                operator,
                right,
            } => {
                if matches!(operator, BinaryOperator::LogicalAnd) {
                    let left = left.evaluate(scope)?.as_bool("il lato sinistro di &&")?;
                    return if left {
                        Ok(Literal::Bool(right.evaluate(scope)?.as_bool("il lato destro di &&")?))
                    } else {
                        Ok(Literal::Bool(false))
                    };
                }
                if matches!(operator, BinaryOperator::LogicalOr) {
                    let left = left.evaluate(scope)?.as_bool("il lato sinistro di ||")?;
                    return if left {
                        Ok(Literal::Bool(true))
                    } else {
                        Ok(Literal::Bool(right.evaluate(scope)?.as_bool("il lato destro di ||")?))
                    };
                }
                operator.evaluate(left.evaluate(scope)?, right.evaluate(scope)?)
            }
            Self::If {
                condition,
                then_value,
                else_value,
            } => {
                if condition.evaluate(scope)?.as_bool("la condizione di if")? {
                    then_value.evaluate(scope)
                } else {
                    else_value.evaluate(scope)
                }
            }
        }
    }
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
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Remainder | Self::Power => {
                self.evaluate_math(left, right)
            }
            Self::Equal | Self::NotEqual | Self::Greater | Self::Less | Self::GreaterEqual | Self::LessEqual => {
                self.evaluate_comparison(left, right)
            }
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::ShiftLeft | Self::ShiftRight => {
                self.evaluate_bitwise(left, right)
            }
            Self::LogicalAnd | Self::LogicalOr => unreachable!("gli operatori logici sono short-circuit"),
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
