// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

//! Solidity type-name parsing
use crate::error::*;
use logos::{Lexer, Logos};
use std::{fmt, result};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Address,
    Uint,
    Int,
    String,
    Bool,
    Bytes,
    Byte(u8),
    Custom(String),
    Array {
        length: Option<u64>,
        inner: Box<Type>,
    },
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
pub enum Token {
    #[token("bool")]
    TypeBool,

    #[token("address")]
    TypeAddress,

    #[token("string")]
    TypeString,

    #[regex("byte|bytes[1-2][0-9]?|bytes3[0-2]?|bytes[4-9]", validate_bytes)]
    TypeByte(u8),

    #[token("bytes")]
    TypeBytes,

    #[regex("int(8|16|24|32|40|48|56|64|72|80|88|96|104|112|120|128|136|144)")]
    #[regex("int(152|160|168|176|184|192|200|208|216|224|232|240|248|256)")]
    #[token("int")]
    TypeInt,

    #[regex("uint(8|16|24|32|40|48|56|64|72|80|88|96|104|112|120|128|136|144)")]
    #[regex("uint(152|160|168|176|184|192|200|208|216|224|232|240|248|256)")]
    #[token("uint")]
    TypeUint,

    #[token("[]")]
    Array,

    #[regex("[a-zA-Z_$][a-zA-Z0-9_$]*")]
    Identifier,

    #[regex("\\[[0-9]+\\]", |lex| lex.slice()[1..lex.slice().len()-1].parse::<u64>().ok() )]
    SizedArray(u64),

    #[error]
    Error,
}

fn validate_bytes(lex: &mut Lexer<Token>) -> Option<u8> {
    let slice = lex.slice().as_bytes();

    if slice.len() > 5 {
        if let Some(byte) = slice.get(6) {
            return Some((slice[5] - b'0') * 10 + (byte - b'0'));
        }
        return Some(slice[5] - b'0');
    } else {
        return Some(1);
    }
}

impl From<Type> for String {
    fn from(field_type: Type) -> String {
        match field_type {
            Type::Address => "address".into(),
            Type::Uint => "uint".into(),
            Type::Int => "int".into(),
            Type::String => "string".into(),
            Type::Bool => "bool".into(),
            Type::Bytes => "bytes".into(),
            Type::Byte(len) => format!("bytes{}", len),
            Type::Custom(custom) => custom,
            Type::Array { inner, length } => {
                let inner: String = (*inner).into();
                match length {
                    None => format!("{}[]", inner),
                    Some(length) => format!("{}[{}]", inner, length),
                }
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        let item: String = self.clone().into();
        write!(f, "{}", item)
    }
}

/// the type string is being validated before it's parsed.
pub fn parse_type(field_type: &str) -> Result<Type> {
    let mut lex = Token::lexer(field_type);

    let mut token = None;
    let mut array_depth = 0;

    while let Some(current_token) = lex.next() {
        let type_ = match current_token {
            Token::Identifier => Type::Custom(lex.slice().to_owned()),
            Token::TypeByte(len) => Type::Byte(len),
            Token::TypeBytes => Type::Bytes,
            Token::TypeBool => Type::Bool,
            Token::TypeUint => Type::Uint,
            Token::TypeInt => Type::Int,
            Token::TypeString => Type::String,
            Token::TypeAddress => Type::Address,
            Token::Array | Token::SizedArray(_) if array_depth == 10 => {
                return Err(ErrorKind::UnsupportedArrayDepth)?;
            }
            Token::SizedArray(len) => {
                token = Some(Type::Array {
                    inner: Box::new(token.expect("if statement checks for some; qed")),
                    length: Some(len),
                });
                array_depth += 1;
                continue;
            }
            Token::Array => {
                token = Some(Type::Array {
                    inner: Box::new(token.expect("if statement checks for some; qed")),
                    length: None,
                });
                array_depth += 1;
                continue;
            }
            Token::Error => {
                return Err(ErrorKind::UnexpectedToken(
                    lex.slice().to_owned(),
                    field_type.to_owned(),
                ))?;
            }
        };

        token = Some(type_);
    }

    Ok(token.ok_or(ErrorKind::NonExistentType)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        let source = "byte[][][7][][][][][][][]";
        parse_type(source).unwrap();
    }

    #[test]
    fn test_nested_array() {
        let source = "byte[][][7][][][][][][][][]";
        assert_eq!(parse_type(source).is_err(), true);
    }

    #[test]
    fn test_malformed_array_type() {
        let source = "byte[7[]uint][]";
        assert_eq!(parse_type(source).is_err(), true)
    }
}
