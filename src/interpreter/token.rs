// New Grammar
// Program = (varDecl | statement)*
// varDecl = Ident ":=" expression ";"
//
// statement = expression | whileDoStmt | doWhileStmt | printStmt | ifStmt | returnStmt | fmcStmt | "break"
// whileDoStmt = "while" "(" expression ")" "do" statement
// doWhileStmt = "do" statement "while" "(" expression ")"
// printStmt = "print" expression ";"
// returnStmt = "return" expression? ";"
// ifStmt = "if" "(" expression ")" statement ("else" statement)?
// fmcStmt = "{" sequence "}"
//
// sequence = term ((";" [choice "->"] term) | ("^" choice))*
// term = variable | "[" term "]" [location] "." term | [location] "<" Variable ">" "." term | choice
// variable = IDENT
// location = "_" | IDENT
// choice = NUMBER
//
// expression = logic_or
// logic_or = logic_and ("||" logic_and )*
// logic_and = equality ( "&&" equality )*
// equality = comparison ( ( "!=" | "==" ) comparison )*
// comparison = sum ( (">" | ">=" | "<" | "<=" ) sum) *
// sum = factor ( ( "-" | "+" ) factor) *
// factor = unary ( ( "/" | "*" ) unary )*
// unary = ( "!" | "-" ) unary | primary
// primary = "true" | "false" | NUMBER | "$" IDENT
// NUMBER = DIGIT+;
// IDENT = ALPHA (ALPHA | DIGIT )*
// ALPHA = [a-zA-Z]
// DIGIT = [0-9]

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

// Lexer based from https://craftinginterpreters.com/scanning.html
#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Identifier(String),
    Integer(i32),

    Assignment, // :=
    SemiColon,  // ;
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Arrow,
    Caret,
    RAngleBracket,
    LAngleBracket,
    Or,
    And,
    NotEqual,
    Equal,
    GreaterThanEqual,
    LessThanEqual,
    Plus,
    Minus,
    Divide,
    Modulo,
    Not,
    Dollar,
    Dot,
    Backslash,

    Star,
    Skip,

    While,
    Do,
    Print,
    Input,
    Int,
    Return,
    If,
    Else,
    True,
    False,
    Break,
    Fn,
    Comma,
    // Input,
    // Sample,
    EOF,
}

impl TokenType {
    pub fn is_choice(&self) -> bool {
        matches!(
            self,
            TokenType::Integer(_)
                | TokenType::True
                | TokenType::False
                | TokenType::Break
                | TokenType::Return
                | TokenType::Skip
        )
    }
}
