// Grammar
// Program      = (declaration)* EOF
//
// declaration  = funDecl | statement
// funDecl      = "fn" function
//
// function     = IDENT "(" parameters? ")" block
// parameters   = IDENT ( "," IDENT )*
// arguments    = expression ( "," expression )*
//
// statement    = exprStmt | whileDoStmt | doWhileStmt | printStmt | ifStmt | block | breakStmt | returnStmt
// exprStmt     = expression ";"
// whileDoStmt  = "while" "(" expression ")" statement
// doWhileStmt  = "do" statement "while" "(" expression ")"
// printStmt    = "print" expression ";"
// breakStmt    = "break" ";"
// returnStmt   = "return" expression? ";"
// ifStmt       = "if" "(" expression ")" statement ("else" statement)?
// block        = "{" (statement)* "}"
// sequence     = loop_term (";" (choice "->")? sequence)?
// loop_term    = atom ("^" Choice)*
// atom         = "(" sequence ")" | Variable | "[" sequence "]" [Location] "." loop_term | [Location] "<" Variable ">" "." loop_term | Choice | Special
// variable     = IDENT
// location     = "_" | IDENT
// choice       = NUMBER | "*" | true | false | break | return
//
// expression   = assignment
// assignment   = IDENT ":=" assignment | logic_or
// logic_or     = logic_and ("||" logic_and )*
// logic_and    = equality ( "&&" equality )*
// equality     = comparison ( ( "!=" | "==" ) comparison )*
// comparison   = sum ( (">" | ">=" | "<" | "<=" ) sum) *
// sum          = factor ( ( "-" | "+" ) factor) *
// factor       = unary ( ( "/" | "*" | "%" ) unary )*
// unary        = ( "!" | "-" | "int" ) unary | call
// call         = primary | IDENT ( "(" arguments? ")" )
// primary      = "true" | "false" | "input" | NUMBER | "$" IDENT | "(" expression ")" | embeddedFmc
// embeddedFmc  = "\" sequence "\"
// NUMBER       = DIGIT+;
// IDENT        = ALPHA (ALPHA | DIGIT | "_" )*
// ALPHA        = [a-zA-Z]
// DIGIT        = [0-9]

use std::fmt::Display;

use crate::{
    fmc_core::{
        Choice, Location, Operation, Special, Term,
        choice::{Constant, Exception},
    },
    interpreter::{Token, TokenType},
};

pub struct Parser {
    tokens: Vec<Token>,
    current_index: usize,
    functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct ParserError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub found: Token,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.found.lexeme.is_empty() {
            write!(
                f,
                "{} (found EOF)\n  at line {}, column {}",
                self.message, self.line, self.column
            )
        } else {
            write!(
                f,
                "{} (found `{}`)\n  at line {}, column {}",
                self.message, self.found.lexeme, self.line, self.column
            )
        }
    }
}

impl std::error::Error for ParserError {}

type ParseResult<T> = Result<T, ParserError>;

#[derive(Debug)]
struct Function {
    name: String,
    parameters: Vec<String>,
    term: Option<Term>,
    recurses: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current_index: 0,
            functions: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> ParseResult<Term> {
        self.program()
    }

    // Program = (declaration)*
    // declaration = funDecl | statement
    fn program(&mut self) -> ParseResult<Term> {
        let mut prog = Vec::new();

        while !self.at_end() {
            if self.current().token_type == TokenType::Fn {
                self.fn_decl()?;
            } else {
                prog.push(self.statement()?);
            }
        }

        // Construct program term from statements

        if let Some(term) = prog.pop() {
            let mut prog_term = term;

            while let Some(term) = prog.pop() {
                prog_term = Term::Case {
                    term: Box::new(term),
                    exit: Choice::skip(),
                    then: Box::new(prog_term),
                }
            }

            // Substitute functions
            for function in self.functions.iter().rev() {
                let mut function_term = function.term.clone().unwrap();

                for para in &function.parameters {
                    function_term = Term::Operation(Operation::Sequence {
                        first: Box::new(Term::Operation(Operation::Update {
                            location: Location::Local(para.clone()),
                            argument: Box::new(Term::Application {
                                function: Box::new(Term::Choice(Choice::skip())),
                                argument: Box::new(Term::Variable { name: para.clone() }),
                                location: Location::Main,
                            }),
                        })),
                        second: Box::new(function_term),
                    })
                    .expand_operations()
                }

                for (i, para) in function.parameters.iter().enumerate() {
                    if i == function.parameters.len() - 1 {
                        let new_name = format!("{}_entry", para); // mark outermost param only

                        function_term = Term::Abstraction {
                            binds: new_name.clone(),
                            term: Box::new(
                                function_term.substitute(para, Term::Variable { name: new_name }),
                            ),
                            location: Location::Main,
                        }
                    } else {
                        function_term = Term::Abstraction {
                            binds: para.clone(),
                            term: Box::new(function_term),
                            location: Location::Main,
                        }
                    };
                }

                let function_term = if function.recurses {
                    let inner = Term::Abstraction {
                        binds: format!("{}_2_f", function.name.clone()),
                        term: Box::new(function_term.clone().substitute(
                            &format!("{}_f", function.name.clone()),
                            Term::Variable {
                                name: format!("{}_2_f", function.name.clone()),
                            },
                        )),
                        location: Location::Main,
                    };
                    Term::Application {
                        function: Box::new(Term::y()),
                        argument: Box::new(inner),
                        location: Location::Main,
                    }
                } else {
                    function_term
                };

                prog_term = Term::Abstraction {
                    binds: format!("{}_f", function.name.clone()),
                    term: Box::new(prog_term),
                    location: Location::Main,
                };

                prog_term = Term::Application {
                    function: Box::new(prog_term),
                    argument: Box::new(function_term),
                    location: Location::Main,
                }
            }

            Ok(prog_term)
        } else {
            Err(self.error_here("expected at least one declaration or statement".to_string()))
        }
    }

    // funDecl = "fn" function
    fn fn_decl(&mut self) -> ParseResult<()> {
        self.consume(TokenType::Fn, "expected `fn` to begin function declaration")?;

        self.function()
    }

    // function = IDENT "(" parameters? ")" block
    fn function(&mut self) -> ParseResult<()> {
        if let TokenType::Identifier(name) = self.current().token_type {
            self.advance();

            self.consume(TokenType::LParen, "expected `(` after function name")?;

            let mut parameters = Vec::new();

            if self.current().token_type != TokenType::RParen {
                parameters = self.parameters()?;
            }

            self.consume(TokenType::RParen, "expected `)` after function parameters")?;

            self.functions.push(Function {
                name: name.clone(),
                parameters: parameters.clone(),
                term: None,
                recurses: false,
            });

            let body = self.block()?.expand_operations();

            let mut term = body;

            term = term.switch_cell_to_local();

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::Exception(Exception::Return),
                then: Box::new(Term::id()),
            };

            for e in &mut self.functions {
                if e.name == name {
                    e.term = Some(term.clone());
                }
            }
            Ok(())
        } else {
            Err(self.error_here("expected function name after `fn`".to_string()))
        }
    }

    // parameters = IDENT ( "," IDENT )*
    fn parameters(&mut self) -> ParseResult<Vec<String>> {
        let mut params = Vec::new();

        if let TokenType::Identifier(first) = self.current().token_type {
            self.advance();
            params.push(first);

            while self.current().token_type == TokenType::Comma {
                self.consume(TokenType::Comma, "expected `,` between parameters")?;

                if let TokenType::Identifier(next) = self.current().token_type {
                    self.advance();

                    params.push(next);
                } else {
                    return Err(self.error_here("expected parameter name after `,`".to_string()));
                }
            }
        } else {
            return Err(self.error_here("expected parameter name".to_string()));
        }

        Ok(params)
    }

    // statement = exprStmt | whileDoStmt | doWhileStmt | printStmt | ifStmt | block | breakStmt | returnStmt
    fn statement(&mut self) -> ParseResult<Term> {
        match self.current().token_type {
            TokenType::While => self.while_do(),
            TokenType::Do => self.do_while(),
            TokenType::Print => self.print_stmt(),
            TokenType::LBrace => self.block(),
            TokenType::If => self.if_stmt(),
            TokenType::Break => self.break_stmt(),
            TokenType::Return => self.return_stmt(),
            _ => self.expr_stmt(),
        }
    }

    // returnStmt = "return" expression? ";"
    fn return_stmt(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::Return, "expected `return`")?;

        if self.current().token_type != TokenType::SemiColon {
            let expr = self.expression()?;

            self.consume(TokenType::SemiColon, "expected `;` after return expression")?;

            Ok(Term::Application {
                function: Box::new(Term::Choice(Choice::Exception(Exception::Return))),
                argument: Box::new(expr),
                location: Location::Main,
            })
        } else {
            self.consume(TokenType::SemiColon, "expected `;` after `return`")?;

            Ok(Term::Application {
                function: Box::new(Term::Choice(Choice::Exception(Exception::Return))),
                argument: Box::new(Term::Choice(Choice::skip())),
                location: Location::Main,
            })
        }
    }

    // breakStmt = "break" ";"
    fn break_stmt(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::Break, "expected `break`")?;

        self.consume(TokenType::SemiColon, "expected `;` after `break`")?;

        Ok(Term::Choice(Choice::Exception(Exception::Break)))
    }

    // doWhileStmt = "do" statement "while" "(" expression ")"
    fn do_while(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::Do, "expected `do`")?;

        let inside = self.statement()?;

        self.consume(TokenType::While, "expected `while` after `do` body")?;

        self.consume(TokenType::LParen, "expected `(` after `while`")?;

        let condition = Box::new(self.expression()?);

        self.consume(TokenType::RParen, "expected `)` after do-while condition")?;

        Ok(Term::Operation(Operation::DoWhile {
            term: Box::new(inside),
            condition,
        }))
    }

    // ifStmt = "if" "(" expression ")" statement ("else" statement)?
    fn if_stmt(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::If, "expected `if`")?;
        self.consume(TokenType::LParen, "expected `(` after `if`")?;

        let condition = Box::new(self.expression()?);

        self.consume(TokenType::RParen, "expected `)` after if condition")?;

        let then = Box::new(self.statement()?);

        if self.current().token_type == TokenType::Else {
            self.advance();

            let else_ = Box::new(self.statement()?);

            Ok(Term::Operation(Operation::IfThenElse {
                condition,
                then,
                else_,
            })
            .expand_operations())
        } else {
            Ok(Term::Operation(Operation::IfThenElse {
                condition,
                then,
                else_: Box::new(Term::Choice(Choice::skip())),
            })
            .expand_operations())
        }
    }

    // printStmt = "print" expression ";"
    fn print_stmt(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::Print, "expected `print`")?;

        let expression = self.expression()?;

        let term = Operation::Write {
            argument: Box::new(expression),
        }
        .expand();

        self.consume(TokenType::SemiColon, "expected `;` after print statement")?;

        Ok(term)
    }

    // whileDoStmt = "while" "(" expression ")" statement
    fn while_do(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::While, "expected `while`")?;

        self.consume(TokenType::LParen, "expected `(` after `while`")?;

        let condition = Box::new(self.expression()?);

        self.consume(TokenType::RParen, "expected `)` after while condition")?;

        let inside = self.statement()?;

        Ok(Term::Operation(Operation::WhileDo {
            condition,
            term: Box::new(inside),
        }))
    }

    fn block(&mut self) -> ParseResult<Term> {
        self.consume(TokenType::LBrace, "expected `{` to begin block")?;

        let mut block = Vec::new();

        while self.current().token_type != TokenType::RBrace {
            if self.at_end() {
                return Err(self.error_here("expected `}` to close block".to_string()));
            }
            block.push(self.statement()?);
        }

        self.advance();

        // Construct program term from declarations and statements

        if let Some(term) = block.pop() {
            let mut block_term = term;

            while let Some(term) = block.pop() {
                block_term = Term::Case {
                    term: Box::new(term),
                    exit: Choice::skip(),
                    then: Box::new(block_term),
                }
            }

            Ok(block_term)
        } else {
            Ok(Term::Choice(Choice::skip()))
        }
    }

    // exprStmt = expression ";"
    fn expr_stmt(&mut self) -> ParseResult<Term> {
        let expr = self.expression()?;

        self.consume(TokenType::SemiColon, "expected `;` after expression")?;

        Ok(expr)
    }

    // expression = assignment
    fn expression(&mut self) -> ParseResult<Term> {
        self.assignment()
    }

    // assignment = IDENT ":=" assignment | logic_or
    // (\f.M)(\x.fn)
    // print (\x.fn)([6].*)
    fn assignment(&mut self) -> ParseResult<Term> {
        if self.lookahead().token_type == TokenType::Assignment {
            if let TokenType::Identifier(ident) = self.current().token_type {
                self.advance();

                if self.current().token_type == TokenType::Assignment {
                    self.advance();

                    let expr = self.assignment()?;

                    Ok(Term::Operation(Operation::Update {
                        location: Location::Cell(ident),
                        argument: Box::new(expr),
                    }))
                } else {
                    Err(self.error_here("expected `:=` in assignment".to_string()))
                }
            } else {
                Err(self.error_here("expected identifier on left side of assignment".to_string()))
            }
        } else {
            self.logic_or()
        }
    }

    // logic_or = logic_and ("||" logic_and )*
    fn logic_or(&mut self) -> ParseResult<Term> {
        let mut term = self.logic_and()?;

        while self.current().token_type == TokenType::Or {
            self.advance();
            let next = self.logic_and()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(Term::Special(Special::LogicOr)),
                }),
            }
        }

        Ok(term)
    }

    // logic_and = equality ( "&&" equality )*
    fn logic_and(&mut self) -> ParseResult<Term> {
        let mut term = self.equality()?;

        while self.current().token_type == TokenType::And {
            self.advance();
            let next = self.equality()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(Term::Special(Special::LogicAnd)),
                }),
            }
        }

        Ok(term)
    }

    // equality = comparison ( ( "!=" | "==" ) comparison )*
    fn equality(&mut self) -> ParseResult<Term> {
        let mut term = self.comparison()?;

        while self.current().token_type == TokenType::NotEqual
            || self.current().token_type == TokenType::Equal
        {
            let operator = match self.current().token_type {
                TokenType::NotEqual => Term::Special(Special::NotEqual),
                TokenType::Equal => Term::Special(Special::Equal),
                _ => panic!(),
            };

            self.advance();
            let next = self.comparison()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(operator),
                }),
            }
        }

        Ok(term)
    }

    // comparison = sum ( (">" | ">=" | "<" | "<=" ) sum) *
    fn comparison(&mut self) -> ParseResult<Term> {
        let mut term = self.sum()?;

        while self.current().token_type == TokenType::LAngleBracket
            || self.current().token_type == TokenType::RAngleBracket
            || self.current().token_type == TokenType::GreaterThanEqual
            || self.current().token_type == TokenType::LessThanEqual
        {
            let operator = match self.current().token_type {
                TokenType::LAngleBracket => Term::Special(Special::LessThan),
                TokenType::RAngleBracket => Term::Special(Special::GreaterThan),
                TokenType::GreaterThanEqual => Term::Special(Special::GreaterThanEqual),
                TokenType::LessThanEqual => Term::Special(Special::LessThanEqual),
                _ => panic!(),
            };

            self.advance();
            let next = self.sum()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(operator),
                }),
            }
        }

        Ok(term)
    }

    // sum = factor ( ( "-" | "+" ) factor) *
    fn sum(&mut self) -> ParseResult<Term> {
        let mut term = self.factor()?;

        while self.current().token_type == TokenType::Minus
            || self.current().token_type == TokenType::Plus
        {
            let operator = match self.current().token_type {
                TokenType::Minus => Term::Special(Special::Subraction),
                TokenType::Plus => Term::Special(Special::Addition),
                _ => panic!(),
            };

            self.advance();
            let next = self.factor()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(operator),
                }),
            }
        }

        Ok(term)
    }

    // factor = unary ( ( "/" | "*" | "%" ) unary )*
    fn factor(&mut self) -> ParseResult<Term> {
        let mut term = self.unary()?;

        while self.current().token_type == TokenType::Divide
            || self.current().token_type == TokenType::Star
            || self.current().token_type == TokenType::Modulo
        {
            let operator = match self.current().token_type {
                TokenType::Divide => Term::Special(Special::Division),
                TokenType::Star => Term::Special(Special::Multiplication),
                TokenType::Modulo => Term::Special(Special::Modulo),
                _ => panic!(),
            };

            self.advance();
            let next = self.unary()?;

            term = Term::Case {
                term: Box::new(term),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(next),
                    exit: Choice::skip(),
                    then: Box::new(operator),
                }),
            }
        }

        Ok(term)
    }

    // unary = ( "!" | "-" | "int" ) unary | call
    fn unary(&mut self) -> ParseResult<Term> {
        let mut operators = Vec::new();

        while self.current().token_type == TokenType::Not
            || self.current().token_type == TokenType::Minus
            || self.current().token_type == TokenType::Int
        {
            operators.push(self.current().token_type);

            self.advance();
        }

        let mut term = self.call()?;

        while let Some(operator) = operators.pop() {
            term = match operator {
                TokenType::Not => Term::Case {
                    term: Box::new(term),
                    exit: Choice::skip(),
                    then: Box::new(Term::Special(Special::LogicNot)),
                },
                TokenType::Minus => Term::Case {
                    term: Box::new(
                        Term::Choice(Choice::Constant(Constant::Integer(0))).as_expression(),
                    ),
                    exit: Choice::skip(),
                    then: Box::new(Term::Case {
                        term: Box::new(term),
                        exit: Choice::skip(),
                        then: Box::new(Term::Special(Special::Subraction)),
                    }),
                },
                TokenType::Int => Term::Case {
                    term: Box::new(term),
                    exit: Choice::skip(),
                    then: Box::new(Term::Special(Special::IntCast)),
                },
                _ => panic!(),
            };
        }

        Ok(term)
    }

    // call = primary | IDENT "(" arguments? ")"
    fn call(&mut self) -> ParseResult<Term> {
        if let TokenType::Identifier(fn_name) = self.current().token_type {
            self.advance();

            self.consume(
                TokenType::LParen,
                "expected `(` after function name in call",
            )?;

            if let Some(function) = self
                .functions
                .iter_mut()
                .find(|e| e.name == fn_name.clone())
            {
                if function.term.is_none() {
                    function.recurses = true;
                }

                let params = function.parameters.clone();
                let mut args = Vec::new();

                if self.current().token_type != TokenType::RParen {
                    args = self.arguments()?;
                }
                self.consume(TokenType::RParen, "expected `)` after function arguments")?;

                if args.len() != params.len() {
                    return Err(self.error_here(format!(
                        "expected {} argument(s) for function `{}`, found {}",
                        params.len(),
                        fn_name,
                        args.len()
                    )));
                }

                let mut term = Term::Variable {
                    name: format!("{}_f", fn_name.clone()),
                };

                while let Some(arg) = args.pop() {
                    term = Term::Operation(Operation::Sequence {
                        first: Box::new(arg),
                        second: Box::new(term.clone()),
                    })
                    .expand_operations();
                }

                Ok(term)
            } else {
                Err(self.error_here(format!("unknown function `{}`", fn_name)))
            }
        } else {
            self.primary()
        }
    }

    // arguments = expression ( "," expression )*
    fn arguments(&mut self) -> ParseResult<Vec<Term>> {
        let mut params = Vec::new();

        let first = self.expression()?;

        params.push(first);

        while self.current().token_type == TokenType::Comma {
            self.consume(TokenType::Comma, "expected `,` between arguments")?;

            let next = self.expression()?;

            params.push(next);
        }

        Ok(params)
    }

    // primary = "true" | "false" | "input" | NUMBER | "$" IDENT | "(" expression ")" | embeddedFmc
    // embeddedFmc = "\" sequence "\"
    // NUMBER = DIGIT+;
    // IDENT = ALPHA (ALPHA | DIGIT | "_" )*
    // ALPHA = [a-zA-Z]
    // DIGIT = [0-9]
    fn primary(&mut self) -> ParseResult<Term> {
        let (term, should_advance) = match self.current().token_type {
            TokenType::True => (
                Term::Choice(Choice::Constant(Constant::Boolean(true))).as_expression(),
                true,
            ),
            TokenType::False => (
                Term::Choice(Choice::Constant(Constant::Boolean(false))).as_expression(),
                true,
            ),
            TokenType::Input => (Term::Operation(Operation::Read), true),
            TokenType::Integer(num) => (
                Term::Choice(Choice::Constant(Constant::Integer(num))).as_expression(),
                true,
            ),
            TokenType::Dollar => {
                self.advance();
                if let TokenType::Identifier(ident) = self.current().token_type {
                    (Term::Operation(Operation::Lookup { cell: ident }), true)
                } else {
                    return Err(self.error_here("expected identifier after `$`".to_string()));
                }
            }
            TokenType::LParen => {
                self.advance();

                let expr = self.expression()?;

                if self.current().token_type != TokenType::RParen {
                    return Err(self.error_here("expected `)` after expression".to_string()));
                }

                (expr, true)
            }
            TokenType::Backslash => (self.embedded_fmc()?, false),
            _ => return Err(self.error_here("expected expression".to_string())),
        };

        if should_advance {
            self.advance();
        }
        Ok(term)
    }

    fn embedded_fmc(&mut self) -> ParseResult<Term> {
        self.consume(
            TokenType::Backslash,
            "expected `\\` to begin embedded FMC term",
        )?;
        let term = self.sequence()?;
        self.consume(
            TokenType::Backslash,
            "expected closing `\\` after embedded FMC term",
        )?;
        Ok(term)
    }

    // sequence = loop_term (";" (choice "->")? sequence)?
    fn sequence(&mut self) -> ParseResult<Term> {
        let left = self.loop_term()?;

        if self.current().token_type == TokenType::SemiColon {
            self.advance();

            let (exit, then) = if self.current().token_type.is_choice()
                && self.lookahead().token_type == TokenType::Arrow
            {
                let exit = self.choice()?;
                self.consume(
                    TokenType::Arrow,
                    "expected `->` after choice in case branch",
                )?;
                (exit, self.sequence()?)
            } else {
                (Choice::skip(), self.sequence()?)
            };

            Ok(Term::Case {
                term: Box::new(left),
                exit,
                then: Box::new(then),
            })
        } else {
            Ok(left)
        }
    }

    // loop_term = atom ("^" Choice)*
    fn loop_term(&mut self) -> ParseResult<Term> {
        let mut term = self.atom()?;

        while self.current().token_type == TokenType::Caret {
            self.advance();
            let branch = self.choice()?;

            term = Term::Loop {
                term: Box::new(term),
                branch,
            };
        }

        Ok(term)
    }

    // atom = "(" sequence ")" | Variable | "[" sequence "]" [Location] "." loop_term | [Location] "<" Variable ">" "." loop_term | Choice | Special
    fn atom(&mut self) -> ParseResult<Term> {
        match self.current().token_type {
            TokenType::RAngleBracket => {
                self.advance();
                return Ok(Term::Special(Special::GreaterThan));
            }
            TokenType::Or => {
                self.advance();
                return Ok(Term::Special(Special::LogicOr));
            }
            TokenType::And => {
                self.advance();
                return Ok(Term::Special(Special::LogicAnd));
            }
            TokenType::NotEqual => {
                self.advance();
                return Ok(Term::Special(Special::NotEqual));
            }
            TokenType::Equal => {
                self.advance();
                return Ok(Term::Special(Special::Equal));
            }
            TokenType::GreaterThanEqual => {
                self.advance();
                return Ok(Term::Special(Special::GreaterThanEqual));
            }
            TokenType::LessThanEqual => {
                self.advance();
                return Ok(Term::Special(Special::LessThanEqual));
            }
            TokenType::Plus => {
                self.advance();
                return Ok(Term::Special(Special::Addition));
            }
            TokenType::Minus => {
                self.advance();
                return Ok(Term::Special(Special::Subraction));
            }
            TokenType::Divide => {
                self.advance();
                return Ok(Term::Special(Special::Division));
            }
            TokenType::Modulo => {
                self.advance();
                return Ok(Term::Special(Special::Modulo));
            }
            TokenType::Not => {
                self.advance();
                return Ok(Term::Special(Special::LogicNot));
            }
            TokenType::Star => {
                self.advance();
                return Ok(Term::Special(Special::Multiplication));
            }
            TokenType::Int => {
                self.advance();
                return Ok(Term::Special(Special::IntCast));
            }
            _ => {}
        }

        match self.current().token_type {
            TokenType::LParen => {
                self.consume(TokenType::LParen, "expected `(`")?;

                let term = self.sequence()?;

                self.consume(TokenType::RParen, "expected `)` after sequence")?;

                Ok(term)
            }
            TokenType::LBracket => {
                // In Application

                self.advance();
                let argument = self.sequence()?;

                self.consume(
                    TokenType::RBracket,
                    "expected `]` after application argument",
                )?;

                let location = if self.current().token_type == TokenType::Dot {
                    Location::Main
                } else {
                    self.location()?
                };

                self.consume(TokenType::Dot, "expected `.` after application location")?;

                let function = self.loop_term()?;

                Ok(Term::Application {
                    function: Box::new(function),
                    argument: Box::new(argument),
                    location,
                })
            }
            TokenType::LAngleBracket => {
                if matches!(self.lookahead().token_type, TokenType::Identifier(_))
                    && self.lookahead_n(2).token_type == TokenType::RAngleBracket
                {
                    self.abstraction()
                } else {
                    self.advance();
                    Ok(Term::Special(Special::LessThan))
                }
            }
            token if token.is_choice() => Ok(self.choice()?.as_term()),
            TokenType::Identifier(ident) => match self.lookahead().token_type {
                TokenType::LAngleBracket => self.abstraction(),
                _ => {
                    self.advance();
                    Ok(Term::Variable { name: ident })
                }
            },

            _ => Err(self.error_here("expected FMC atom".to_string())),
        }
    }

    // [Location] "<" Variable ">" "." loop_term
    fn abstraction(&mut self) -> ParseResult<Term> {
        let location = if self.current().token_type == TokenType::LAngleBracket {
            Location::Main
        } else {
            self.location()?
        };

        self.consume(
            TokenType::LAngleBracket,
            "expected `<` to begin abstraction",
        )?;

        if let TokenType::Identifier(variable) = self.current().token_type {
            self.advance();

            self.consume(
                TokenType::RAngleBracket,
                "expected `>` after abstraction binder",
            )?;

            self.consume(TokenType::Dot, "expected `.` after abstraction binder")?;

            let term = self.loop_term()?;

            Ok(Term::Abstraction {
                binds: variable,
                term: Box::new(term),
                location,
            })
        } else {
            Err(self.error_here("expected abstraction binder name".to_string()))
        }
    }

    fn location(&mut self) -> ParseResult<Location> {
        let location = match self.current().token_type {
            TokenType::Identifier(ident) => {
                if ident == "in" {
                    Location::Input
                } else if ident == "out" {
                    Location::Output
                } else if ident == "rnd_b" {
                    Location::RndBool
                } else if ident == "rnd_f" {
                    Location::RndFloat
                } else {
                    Location::Cell(ident)
                }
            }
            _ => return Err(self.error_here("expected location name".to_string())),
        };

        self.advance();

        Ok(location)
    }

    fn choice(&mut self) -> ParseResult<Choice> {
        let choice = match self.current().token_type {
            TokenType::Integer(choice) => Choice::Constant(Constant::Integer(choice)),
            TokenType::True => Choice::Constant(Constant::Boolean(true)),
            TokenType::False => Choice::Constant(Constant::Boolean(false)),
            TokenType::Break => Choice::Exception(Exception::Break),
            TokenType::Return => Choice::Exception(Exception::Return),
            TokenType::Skip => Choice::Exception(Exception::Skip),
            _ => return Err(self.error_here("expected choice value".to_string())),
        };

        self.advance();
        Ok(choice)
    }

    fn lookahead(&self) -> Token {
        self.lookahead_n(1)
    }

    fn lookahead_n(&self, n: usize) -> Token {
        self.tokens[self.current_index + n].clone()
    }

    fn current(&self) -> Token {
        self.tokens[self.current_index].clone()
    }

    fn at_end(&self) -> bool {
        self.current().token_type == TokenType::EOF
    }

    fn advance(&mut self) -> Token {
        let prev = self.tokens[self.current_index].clone();
        self.current_index += 1;

        prev
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> ParseResult<Token> {
        if self.current().token_type != token_type {
            return Err(self.error_here(format!("{}; expected {:?}", message, token_type)));
        }
        Ok(self.advance())
    }

    fn error_here(&self, message: String) -> ParserError {
        let token = self.current();
        ParserError {
            line: token.line,
            column: token.column,
            message,
            found: token,
        }
    }
}

#[cfg(test)]
// These tests were generated by ChatGPT.
mod tests {
    use super::{Parser, ParserError};
    use crate::{
        fmc_core::{
            Choice, Location, Operation, Special, Term,
            choice::{Constant, Exception},
        },
        interpreter::{Lexer, TokenType},
    };

    fn parse(source: &str) -> Result<Term, ParserError> {
        let result = Lexer::new(source.to_string()).tokenise();
        assert!(
            result.errors.is_empty(),
            "lexer errors in parser test: {:?}",
            result.errors
        );
        Parser::new(result.tokens).parse()
    }

    fn v(name: &str) -> Term {
        Term::Variable {
            name: name.to_string(),
        }
    }

    fn int_expr(value: i32) -> Term {
        Term::Application {
            function: Box::new(Term::Choice(Choice::skip())),
            argument: Box::new(Term::Choice(Choice::Constant(Constant::Integer(value)))),
            location: Location::Main,
        }
    }

    #[test]
    fn parses_assignment_statement_to_update_operation() {
        let parsed = parse("a := 5;").unwrap();

        assert_eq!(
            parsed,
            Term::Operation(Operation::Update {
                location: Location::Cell("a".to_string()),
                argument: Box::new(int_expr(5)),
            })
        );
    }

    #[test]
    fn parses_expression_precedence_before_statement_termination() {
        let parsed = parse("1 + 2 * 3;").unwrap();

        assert_eq!(
            parsed,
            Term::Case {
                term: Box::new(int_expr(1)),
                exit: Choice::skip(),
                then: Box::new(Term::Case {
                    term: Box::new(Term::Case {
                        term: Box::new(int_expr(2)),
                        exit: Choice::skip(),
                        then: Box::new(Term::Case {
                            term: Box::new(int_expr(3)),
                            exit: Choice::skip(),
                            then: Box::new(Term::Special(Special::Multiplication)),
                        }),
                    }),
                    exit: Choice::skip(),
                    then: Box::new(Term::Special(Special::Addition)),
                }),
            }
        );
    }

    #[test]
    fn parses_multiple_statements_as_sequence_of_cases() {
        let parsed = parse("print 1; break;").unwrap();

        match parsed {
            Term::Case { term, exit, then } => {
                assert_eq!(exit, Choice::skip());
                assert_eq!(*then, Term::Choice(Choice::Exception(Exception::Break)));

                match *term {
                    Term::Case {
                        term: printed,
                        exit: printed_exit,
                        then: continuation,
                    } => {
                        assert_eq!(*printed, int_expr(1));
                        assert_eq!(printed_exit, Choice::skip());
                        match *continuation {
                            Term::Abstraction { location, .. } => {
                                assert_eq!(location, Location::Main);
                            }
                            other => panic!("expected write continuation, got {other:?}"),
                        }
                    }
                    other => panic!("expected write expansion, got {other:?}"),
                }
            }
            other => panic!("expected sequenced statements, got {other:?}"),
        }
    }

    #[test]
    fn parses_fmc_application_with_explicit_location() {
        let parsed = parse("\\ [x]out.y \\;").unwrap();

        assert_eq!(
            parsed,
            Term::Application {
                function: Box::new(v("y")),
                argument: Box::new(v("x")),
                location: Location::Output,
            }
        );
    }

    #[test]
    fn parses_function_declaration_and_call() {
        let parsed = parse(
            "
            fn id(x) { return $x; }
            id(5);
            ",
        )
        .unwrap();

        match parsed {
            Term::Application {
                function,
                argument,
                location,
            } => {
                assert_eq!(location, Location::Main);

                match *function {
                    Term::Abstraction { binds, .. } => {
                        assert_eq!(binds, "id_f");
                    }
                    other => panic!("expected function binding abstraction, got {other:?}"),
                }

                match *argument {
                    Term::Abstraction {
                        binds, location, ..
                    } => {
                        assert_eq!(binds, "x_entry");
                        assert_eq!(location, Location::Main);
                    }
                    other => panic!("expected function term abstraction, got {other:?}"),
                }
            }
            other => panic!("expected substituted function application, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_program() {
        let err = parse("").unwrap_err();

        assert_eq!(err.line, 1);
        assert_eq!(err.column, 1);
        assert_eq!(
            err.message,
            "expected at least one declaration or statement"
        );
        assert_eq!(err.found.token_type, TokenType::EOF);
    }

    #[test]
    fn parses_empty_block_as_skip() {
        let parsed = parse("if (true) {}").unwrap();

        match parsed {
            Term::Case { exit, then, .. } => {
                assert_eq!(exit, Choice::Constant(Constant::Boolean(false)));
                assert_eq!(*then, Term::Choice(Choice::skip()));
            }
            other => panic!("expected if-then-else expansion, got {other:?}"),
        }
    }

    #[test]
    fn reports_missing_semicolon_at_next_token_position() {
        let err = parse("print 1\nprint 2;").unwrap_err();

        assert_eq!(err.line, 2);
        assert_eq!(err.column, 1);
        assert_eq!(
            err.message,
            "expected `;` after print statement; expected SemiColon"
        );
        assert_eq!(err.found.lexeme, "print");
    }

    #[test]
    fn reports_unknown_function_calls() {
        let err = parse("foo();").unwrap_err();

        assert_eq!(err.line, 1);
        assert_eq!(err.column, 5);
        assert_eq!(err.message, "unknown function `foo`");
        assert_eq!(err.found.token_type, TokenType::RParen);
    }

    #[test]
    fn reports_wrong_function_arity() {
        let err = parse(
            "
            fn add(x, y) { return $x; }
            add(1);
            ",
        )
        .unwrap_err();

        assert_eq!(err.line, 3);
        assert_eq!(err.column, 19);
        assert_eq!(
            err.message,
            "expected 2 argument(s) for function `add`, found 1"
        );
        assert_eq!(err.found.lexeme, ";");
    }

    #[test]
    fn parses_input_keyword_as_read_operation() {
        let parsed = parse("a := input;").unwrap();

        assert_eq!(
            parsed,
            Term::Operation(Operation::Update {
                location: Location::Cell("a".to_string()),
                argument: Box::new(Term::Operation(Operation::Read)),
            })
        );
    }

    #[test]
    fn parses_int_cast_as_unary_expression() {
        let parsed = parse("int 4;").unwrap();

        assert_eq!(
            parsed,
            Term::Case {
                term: Box::new(
                    Term::Choice(Choice::Constant(Constant::Integer(4))).as_expression()
                ),
                exit: Choice::skip(),
                then: Box::new(Term::Special(Special::IntCast)),
            }
        );
    }

    #[test]
    fn parses_embedded_fmc_as_expression() {
        let parsed = parse("return \\out<x>.[x].#\\;").unwrap();

        match parsed {
            Term::Application { argument, .. } => match *argument {
                Term::Abstraction { location, .. } => assert_eq!(location, Location::Output),
                other => panic!("expected embedded FMC abstraction, got {other:?}"),
            },
            other => panic!("expected return application, got {other:?}"),
        }
    }
}
