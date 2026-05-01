use std::fmt::Display;

use log::info;

use crate::{
    fmc_core::Term,
    interpreter::{Lexer, Parser, lexer::LexerError, parser::ParserError},
    machines::{
        KrivineMachine, Machine, StackMachine,
        machine::{MachineType, StepResult},
    },
};

pub struct Interpreter {
    silent: bool,
    machine_type: MachineType,
}

#[derive(Debug)]
pub enum CompileError {
    Lexer(Vec<LexerError>),
    Parser(ParserError),
}

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Lexer(errors) => {
                writeln!(f, "Lexer failed with {} error(s)", errors.len())?;
                for (i, error) in errors.iter().enumerate() {
                    if i > 0 {
                        writeln!(f)?;
                        writeln!(f)?;
                    }
                    write!(f, "{error}")?;
                }
                Ok(())
            }
            CompileError::Parser(error) => write!(f, "Parser failed\n{error}"),
        }
    }
}

impl std::error::Error for CompileError {}

const STDLIB_RND: &str = include_str!("rnd.lithe");

impl Interpreter {
    pub fn new(silent: bool, machine_type: MachineType) -> Self {
        Interpreter {
            silent,
            machine_type,
        }
    }

    pub fn compile(
        &self,
        program: String,
        optimise: bool,
        rnd: bool,
    ) -> Result<Term, CompileError> {
        let program = if rnd {
            format!("{STDLIB_RND}\n{program}")
        } else {
            program
        };

        let lexer = Lexer::new(program);
        let result = lexer.tokenise();

        let tokens = result.tokens;
        let errors = result.errors;

        if !errors.is_empty() {
            return Err(CompileError::Lexer(errors));
        }

        let mut parser = Parser::new(tokens);
        let mut term = parser.parse().map_err(CompileError::Parser)?;
        term = term.expand_operations();

        if optimise {
            term = term.compute_reduction(true);
        }

        Ok(term)
    }

    pub fn interpret(
        &self,
        program: String,
        optimise: bool,
        trace: bool,
        rnd: bool,
    ) -> Result<StepResult, CompileError> {
        let term = self.compile(program, optimise, rnd)?;

        Ok(self.run(term, trace))
    }

    pub fn run(&self, term: Term, trace: bool) -> StepResult {
        let mut machine: Box<dyn Machine> = if self.machine_type == MachineType::Stack {
            info!("Using stack machine");
            Box::new(StackMachine::new(term))
        } else {
            info!("Using Krivine machine");
            Box::new(KrivineMachine::new(term))
        };
        machine.set_silent(self.silent);
        machine.run(trace)
    }
}
