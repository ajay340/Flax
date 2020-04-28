use crate::ast::{Binary, Unary, Literal, Grouping, ExprType, Expr};
use crate::errors::{RuntimeError};
use crate::lexer::{TokenType, Token};
use std::fmt;


/// Implement a Visitor for each struct in the Abstract Syntax Tree
pub trait Visitor<E>  {
    fn accept<R, V: Interpreter<R>>(&self, visitor: &V) -> Result<R, E>;
}

impl Visitor<RuntimeError> for Unary {
    fn accept<R, V: Interpreter<R>>(&self, visitor: &V) -> Result<R, RuntimeError> {
        visitor.visit_unary(self)
    }
}

impl Visitor<RuntimeError> for Literal {
    fn accept<R, V: Interpreter<R>>(&self, visitor: &V) -> Result<R, RuntimeError> {
        visitor.visit_literal(self)
    }
}

impl Visitor<RuntimeError> for Binary {
    fn accept<R, V: Interpreter<R>>(&self, visitor: &V) -> Result<R, RuntimeError> {
        visitor.visit_binary(self)
    }
}

impl Visitor<RuntimeError> for Grouping {
    fn accept<R, V: Interpreter<R>>(&self, visitor: &V) -> Result<R, RuntimeError> {
        visitor.visit_grouping(self)
    }
}



pub trait Interpreter<R> {

    fn visit_binary(&self, binary: &Binary) -> Result<R, RuntimeError>;
    fn visit_unary(&self, urnary: &Unary) -> Result<R, RuntimeError>;
    fn visit_literal(&self, literal: &Literal) -> Result<R, RuntimeError>;
    fn visit_grouping(&self, grouping: &Grouping) -> Result<R, RuntimeError>;

}



pub fn interpret_ast(expression: Expr) -> Result<Obj, RuntimeError> {
    match expression.expr {
        ExprType::B(ref val) => expression.visit_binary(val),
        ExprType::G(ref val) => expression.visit_grouping(val),
        ExprType::L(ref val) => expression.visit_literal(val),
        ExprType::U(ref val) => expression.visit_unary(val),
    }
}

macro_rules! evaluate {
    ($e:expr, $sel:ident) => {
        match &$e {
            ExprType::L(lit) => lit.accept($sel),
            ExprType::B(ref b_expr) => b_expr.accept($sel),
            ExprType::U(ref u_expr) => u_expr.accept($sel),
            ExprType::G(ref g_expr) => g_expr.accept($sel),
        }
    };
}

#[derive(PartialEq, Debug)]
pub enum Obj {
    BOOL(bool),
    STRING(String),
    NUMBER(f64),
    Nil,
}

impl fmt::Display for Obj {
    fn fmt(&self, f: &mut fmt::Formatter<>) -> fmt::Result {
        match self {
            Obj::BOOL(val) => write!(f, "{}", val),
            Obj::Nil => write!(f, "nil"),
            Obj::STRING(val) => write!(f, "\"{}\"", val),
            Obj::NUMBER(val) => write!(f, "{}", val),
        }
    }
}

impl Interpreter<Obj> for Expr {
    fn visit_binary(&self, binary: &Binary) -> Result<Obj, RuntimeError> {
        let right: Obj = evaluate!(binary.right.expr, self);
        let left: Obj = evaluate!(binary.left.expr, self);
        
        match binary.operator.token_type {
            TokenType::Minus => check_numbers(left, right, TokenType::Minus),
            TokenType::Plus => check_numbers(left, right, TokenType::Plus),
            TokenType::Star => check_numbers(left, right, TokenType::Star),
            TokenType::Slash => check_numbers(left, right, TokenType::Slash),
            TokenType::PlusPlus => concatenate_values((left, right)),
            TokenType::EqualEqual => determine_equality((left, right), TokenType::EqualEqual),
            TokenType::BangEqual => determine_equality((left, right), TokenType::BangEqual),
            TokenType::Less => determine_int_comparison((left, right), TokenType::Less),
            TokenType::LessEqual => determine_int_comparison((left, right), TokenType::LessEqual),
            TokenType::Greater => determine_int_comparison((left, right), TokenType::Greater),
            TokenType::GreaterEqual => determine_int_comparison((left, right), TokenType::GreaterEqual),
            _ => Err(RuntimeError::new(binary.operator.lexeme, format!("Expeceted expression, given {}", binary.operator.lexeme), binary.operator.line)),
        }
    }

    fn visit_unary(&self, urnary: &Unary) -> Result {
        let expr: Obj = evaluate!(urnary.expr.expr, self);

        match urnary.operator.token_type {
            TokenType::Minus => {
                if let Obj::NUMBER(v) = expr {
                   return Obj::NUMBER(-1.0 * v);
                }
                panic!("Invalid unary expression.  Expected Number")
            },
            TokenType::Bang => Obj::BOOL(!is_truthy(expr)),
            _ => panic!("Invalid token for Unary"),
        }
    }

    fn visit_literal(&self, literal: &Literal) -> Obj {        
        if literal.val.parse::<f64>().is_ok() {
            Obj::NUMBER(literal.val.parse::<f64>().unwrap())
        }
        else if literal.val.parse::<bool>().is_ok() {
            Obj::BOOL(literal.val.parse::<bool>().unwrap())
        }
        else if literal.val.parse::<String>().is_ok() {
            let s = literal.val.parse::<String>().unwrap();
            match &s[..] {
                "nil" => Obj::Nil,
                "true" => Obj::BOOL(true),
                "false" => Obj::BOOL(false),
                _ => Obj::STRING(s)
            }
        }
        else {
            panic!("Parsing error with: Literal")
        }
    }

    fn visit_grouping(&self, grouping: &Grouping) -> Obj {
        evaluate!(grouping.expr.expr, self)
    }
}


fn check_numbers(paris: (Obj, Obj), op: Token) -> Result<Obj, RuntimeError> {
    match paris {
        (Obj::NUMBER(left), Obj::NUMBER(right)) => {
            match op.token_type {
                TokenType::Minus => Ok(Obj::NUMBER(left - right)),
                TokenType::Plus => Ok(Obj::NUMBER(left + right)),
                TokenType::Star => Ok(Obj::NUMBER(left * right)),
                TokenType::Slash => Ok(Obj::NUMBER(left / right)),
                _ => panic!("Error"),
            }
        }
        _ => Err(RuntimeError::new(op.lexeme, format!("Expected Numbers for - given {} {}", paris.0, paris.1), op.line)),
    }
}

// Two cases:
// left and right are strings               =>combine the strings 
// left is a string and right is a int      => combine the string and int into a string

fn concatenate_values(pairs: (Obj, Obj)) -> Obj {
    match pairs {
        (Obj::STRING(mut v), Obj::STRING(v2)) => {
            v.push_str(&v2);
            Obj::STRING(v)
        },
        (Obj::STRING(mut v), Obj::NUMBER(v2)) => {
            v.push_str(&v2.to_string());
            Obj::STRING(v)
        }
        (Obj::NUMBER(v), Obj::STRING(v2)) => {
            let mut s = v.to_string();
            s.push_str(&v2);
            Obj::STRING(s)
        },
        _ => panic!("Expected two strings or a string an a integer"),
    }
}


fn determine_equality(pair: (Obj, Obj), operator: TokenType) -> Obj {
    match operator {
        TokenType::EqualEqual => {
             match pair {
                (Obj::BOOL(v), Obj::BOOL(v2)) => Obj::BOOL(v == v2),
                (Obj::Nil, Obj::Nil) => Obj::BOOL(true),
                (Obj::STRING(v), Obj::STRING(v2)) => Obj::BOOL(v == v2),
                (Obj::NUMBER(v), Obj::NUMBER(v2)) => Obj::BOOL(v == v2),
                _ => Obj::BOOL(false),
            }
        },
        TokenType::BangEqual => {
            match pair {
                (Obj::BOOL(v), Obj::BOOL(v2)) => Obj::BOOL(v != v2),
                (Obj::Nil, Obj::Nil) => Obj::BOOL(false),
                (Obj::STRING(v), Obj::STRING(v2)) => Obj::BOOL(v != v2),
                (Obj::NUMBER(v), Obj::NUMBER(v2)) => Obj::BOOL(v != v2),
                _ => Obj::BOOL(true),
            }
        },
        _ => panic!("Invalid token type. Expected '==' or '!='.")
    }
}

fn determine_int_comparison(pair: (Obj, Obj), operator: TokenType) -> Obj {
    match pair {
        (Obj::NUMBER(val), Obj::NUMBER(val2)) => {
            match operator {
                TokenType::Less => Obj::BOOL(val < val2),
                TokenType::LessEqual => Obj::BOOL(val <= val2),
                TokenType::Greater => Obj::BOOL(val > val2),
                TokenType::GreaterEqual => Obj::BOOL(val >= val2),
                _ => panic!("Expected boolean values")
            } 
        }, 
        _ => panic!("Expected integer values"),
    }
}

// Determines if  a value is truthy or falsy
// Important: Flax follows Ruby's rule: everything but False and nil are true
fn is_truthy(value: Obj) -> bool {
    match value {
        Obj::BOOL(false) | Obj::Nil => false,
        _ => true,
    }
}