use std::{error::Error, fmt::Display};

use crate::interpreter::{Token, TokenType};

pub struct Lexer {
    source: String,
}

#[derive(Debug)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexerError>,
}

#[derive(Debug)]
pub struct LexerError {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    error_type: LexerErrorType,
}

#[derive(Debug)]
enum LexerErrorType {
    UnexpectedToken { lexeme: String },
    IncompleteEqual,
    IncompleteAnd,
    IncompleteOr,
    IncompleteAssignment,
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error_type {
            LexerErrorType::UnexpectedToken { lexeme } => {
                write!(
                    f,
                    "Unexpected token: {}\n  at line {}, column {}",
                    lexeme, self.line, self.column
                )
            }
            LexerErrorType::IncompleteAssignment => {
                write!(
                    f,
                    "Incomplete assignment operator\n  at line {}, column {}",
                    self.line, self.column
                )
            }
            LexerErrorType::IncompleteAnd => {
                write!(
                    f,
                    "Incomplete and operator\n  at line {}, column {}",
                    self.line, self.column
                )
            }
            LexerErrorType::IncompleteEqual => {
                write!(
                    f,
                    "Incomplete equal operator\n  at line {}, column {}",
                    self.line, self.column
                )
            }
            LexerErrorType::IncompleteOr => {
                write!(
                    f,
                    "Incomplete or operator\n  at line {}, column {}",
                    self.line, self.column
                )
            }
        }
    }
}

impl Error for LexerError {}

impl Lexer {
    pub fn new(source: String) -> Self {
        Lexer { source }
    }

    pub fn tokenise(&self) -> LexerResult {
        let mut tokens = Vec::new();
        let mut chars = self.source.chars().peekable();
        let mut errors = Vec::new();
        let mut i = 0;
        let mut line = 1;
        let mut column = 1;

        while let Some(current_char) = chars.next() {
            let mut current_lexeme = current_char.to_string();
            let start_line = line;
            let start_column = column;

            let new_token = match current_char {
                '[' => TokenType::LBracket,
                ']' => TokenType::RBracket,
                '(' => TokenType::LParen,
                ')' => TokenType::RParen,
                '{' => TokenType::LBrace,
                '}' => TokenType::RBrace,
                '.' => TokenType::Dot,
                '^' => TokenType::Caret,
                ';' => TokenType::SemiColon,
                '$' => TokenType::Dollar,
                '*' => TokenType::Star,
                '+' => TokenType::Plus,
                '/' => TokenType::Divide,
                '%' => TokenType::Modulo,
                '\\' => TokenType::Backslash,
                '#' => TokenType::Skip,
                ',' => TokenType::Comma,
                '_' => TokenType::Identifier("_".to_string()),
                '!' => match chars.peek() {
                    Some('=') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::NotEqual
                    }
                    _ => TokenType::Not,
                },
                '<' => match chars.peek() {
                    Some('=') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::LessThanEqual
                    }
                    _ => TokenType::LAngleBracket,
                },
                '>' => match chars.peek() {
                    Some('=') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::GreaterThanEqual
                    }
                    _ => TokenType::RAngleBracket,
                },
                '-' => match chars.peek() {
                    Some('>') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::Arrow
                    }
                    _ => TokenType::Minus,
                },
                ':' => match chars.peek() {
                    Some('=') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::Assignment
                    }
                    _ => {
                        errors.push(LexerError {
                            index: i,
                            line: start_line,
                            column: start_column,
                            error_type: LexerErrorType::IncompleteAssignment,
                        });
                        advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                        continue;
                    }
                },
                '|' => match chars.peek() {
                    Some('|') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::Or
                    }
                    _ => {
                        errors.push(LexerError {
                            index: i,
                            line: start_line,
                            column: start_column,
                            error_type: LexerErrorType::IncompleteOr,
                        });
                        advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                        continue;
                    }
                },
                '&' => match chars.peek() {
                    Some('&') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::And
                    }
                    _ => {
                        errors.push(LexerError {
                            index: i,
                            line: start_line,
                            column: start_column,
                            error_type: LexerErrorType::IncompleteAnd,
                        });
                        advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                        continue;
                    }
                },
                '=' => match chars.peek() {
                    Some('=') => {
                        current_lexeme += &chars.next().unwrap().to_string();
                        TokenType::Equal
                    }
                    _ => {
                        errors.push(LexerError {
                            index: i,
                            line: start_line,
                            column: start_column,
                            error_type: LexerErrorType::IncompleteEqual,
                        });
                        advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                        continue;
                    }
                },
                c if c.is_ascii_whitespace() => {
                    // ignore whitespace
                    advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                    continue;
                }
                _ => {
                    if current_char.is_ascii_digit() {
                        while chars.peek().unwrap_or(&'a').is_ascii_digit() {
                            current_lexeme += &chars.next().unwrap().to_string();
                        }

                        TokenType::Integer(current_lexeme.parse().unwrap())
                    } else if current_char.is_ascii_alphabetic() {
                        while chars
                            .peek()
                            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                        {
                            current_lexeme += &chars.next().unwrap().to_string();
                        }

                        match current_lexeme.as_str() {
                            // "read" => TokenType::Read,
                            "while" => TokenType::While,
                            "do" => TokenType::Do,
                            "print" => TokenType::Print,
                            "input" => TokenType::Input,
                            "int" => TokenType::Int,
                            "return" => TokenType::Return,
                            "if" => TokenType::If,
                            "else" => TokenType::Else,
                            // "sample" => TokenType::Sample,
                            "break" => TokenType::Break,
                            "true" => TokenType::True,
                            "false" => TokenType::False,
                            "fn" => TokenType::Fn,
                            _ => TokenType::Identifier(current_lexeme.clone()),
                        }
                        // TokenType::Identifier(current_lexeme.clone())
                    } else {
                        // Unexpected character.
                        errors.push(LexerError {
                            index: i,
                            line: start_line,
                            column: start_column,
                            error_type: LexerErrorType::UnexpectedToken {
                                lexeme: current_lexeme.clone(),
                            },
                        });
                        advance_position(&current_lexeme, &mut i, &mut line, &mut column);
                        continue;
                    }
                }
            };
            advance_position(&current_lexeme, &mut i, &mut line, &mut column);
            tokens.push(Token {
                token_type: new_token,
                lexeme: current_lexeme,
                line: start_line,
                column: start_column,
            });
        }

        tokens.push(Token {
            token_type: TokenType::EOF,
            lexeme: String::new(),
            line,
            column,
        });

        LexerResult { tokens, errors }
    }
}

fn advance_position(lexeme: &str, index: &mut usize, line: &mut usize, column: &mut usize) {
    for ch in lexeme.chars() {
        *index += 1;
        if ch == '\n' {
            *line += 1;
            *column = 1;
        } else {
            *column += 1;
        }
    }
}

#[cfg(test)]
// These tests were generated by ChatGPT.
mod tests {
    use super::{Lexer, LexerError, LexerErrorType};
    use crate::interpreter::TokenType;

    fn token_types(source: &str) -> Vec<TokenType> {
        Lexer::new(source.to_string())
            .tokenise()
            .tokens
            .into_iter()
            .map(|token| token.token_type)
            .collect()
    }

    fn lexemes(source: &str) -> Vec<String> {
        Lexer::new(source.to_string())
            .tokenise()
            .tokens
            .into_iter()
            .map(|token| token.lexeme)
            .collect()
    }

    fn positions(source: &str) -> Vec<(usize, usize)> {
        Lexer::new(source.to_string())
            .tokenise()
            .tokens
            .into_iter()
            .map(|token| (token.line, token.column))
            .collect()
    }

    fn errors(source: &str) -> Vec<LexerError> {
        Lexer::new(source.to_string()).tokenise().errors
    }

    #[test]
    fn tokenises_punctuation_and_single_character_operators() {
        assert_eq!(
            token_types("[](){}.^;$*/%\\#, +"),
            vec![
                TokenType::LBracket,
                TokenType::RBracket,
                TokenType::LParen,
                TokenType::RParen,
                TokenType::LBrace,
                TokenType::RBrace,
                TokenType::Dot,
                TokenType::Caret,
                TokenType::SemiColon,
                TokenType::Dollar,
                TokenType::Star,
                TokenType::Divide,
                TokenType::Modulo,
                TokenType::Backslash,
                TokenType::Skip,
                TokenType::Comma,
                TokenType::Plus,
                TokenType::EOF,
            ]
        );
    }

    #[test]
    fn tokenises_multi_character_operators_and_fallback_variants() {
        assert_eq!(
            token_types("!= ! <= < >= > -> - := || && =="),
            vec![
                TokenType::NotEqual,
                TokenType::Not,
                TokenType::LessThanEqual,
                TokenType::LAngleBracket,
                TokenType::GreaterThanEqual,
                TokenType::RAngleBracket,
                TokenType::Arrow,
                TokenType::Minus,
                TokenType::Assignment,
                TokenType::Or,
                TokenType::And,
                TokenType::Equal,
                TokenType::EOF,
            ]
        );
    }

    #[test]
    fn tokenises_keywords_identifiers_and_integer_literals() {
        assert_eq!(
            token_types(
                "while do print input int return if else break true false fn abc abc123 rnd_bool _ 42",
            ),
            vec![
                TokenType::While,
                TokenType::Do,
                TokenType::Print,
                TokenType::Input,
                TokenType::Int,
                TokenType::Return,
                TokenType::If,
                TokenType::Else,
                TokenType::Break,
                TokenType::True,
                TokenType::False,
                TokenType::Fn,
                TokenType::Identifier("abc".to_string()),
                TokenType::Identifier("abc123".to_string()),
                TokenType::Identifier("rnd_bool".to_string()),
                TokenType::Identifier("_".to_string()),
                TokenType::Integer(42),
                TokenType::EOF,
            ]
        );
    }

    #[test]
    fn preserves_original_lexemes_for_emitted_tokens() {
        assert_eq!(
            lexemes("print abc123 := 42"),
            vec![
                "print".to_string(),
                "abc123".to_string(),
                ":=".to_string(),
                "42".to_string(),
                String::new(),
            ]
        );
    }

    #[test]
    fn ignores_ascii_whitespace_between_tokens() {
        assert_eq!(
            token_types(" \n\tprint   7\r\nbreak "),
            vec![
                TokenType::Print,
                TokenType::Integer(7),
                TokenType::Break,
                TokenType::EOF,
            ]
        );
    }

    #[test]
    fn reports_incomplete_compound_operators() {
        let result = Lexer::new(": | & =".to_string()).tokenise();

        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].token_type, TokenType::EOF);
        assert_eq!(
            result.errors.iter().map(|error| error.index).collect::<Vec<_>>(),
            vec![
                0, 2, 4, 6,
            ]
        );
        assert_eq!(
            result
                .errors
                .iter()
                .map(|error| (error.line, error.column))
                .collect::<Vec<_>>(),
            vec![(1, 1), (1, 3), (1, 5), (1, 7)]
        );
        assert!(matches!(
            result.errors[0].error_type,
            LexerErrorType::IncompleteAssignment
        ));
        assert!(matches!(result.errors[1].error_type, LexerErrorType::IncompleteOr));
        assert!(matches!(result.errors[2].error_type, LexerErrorType::IncompleteAnd));
        assert!(matches!(
            result.errors[3].error_type,
            LexerErrorType::IncompleteEqual
        ));
    }

    #[test]
    fn reports_unexpected_tokens_and_continues_lexing() {
        let result = Lexer::new("@ print".to_string()).tokenise();

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].index, 0);
        assert_eq!((result.errors[0].line, result.errors[0].column), (1, 1));
        assert!(matches!(
            result.errors[0].error_type,
            LexerErrorType::UnexpectedToken { ref lexeme } if lexeme == "@"
        ));
        assert_eq!(
            result
                .tokens
                .into_iter()
                .map(|token| token.token_type)
                .collect::<Vec<_>>(),
            vec![TokenType::Print, TokenType::EOF]
        );
    }

    #[test]
    fn reports_error_indices_as_character_offsets() {
        let result = errors("abc @");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index, 4);
        assert_eq!(result[0].line, 1);
        assert_eq!(result[0].column, 5);
        assert!(matches!(
            result[0].error_type,
            LexerErrorType::UnexpectedToken { ref lexeme } if lexeme == "@"
        ));
    }

    #[test]
    fn records_token_line_and_column_positions() {
        assert_eq!(
            positions("print 7;\nreturn false;"),
            vec![(1, 1), (1, 7), (1, 8), (2, 1), (2, 8), (2, 13), (2, 14)]
        );
    }
}
