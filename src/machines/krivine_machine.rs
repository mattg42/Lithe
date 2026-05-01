use std::collections::HashMap;

use crate::{
    fmc_core::{Choice, Location, Special, Term, choice::Constant},
    machines::{
        Machine,
        machine::StepResult,
        runtime_io::{
            Number, number_to_term, random_bool_term, random_float_term, read_input_term,
            term_to_number,
        },
    },
};

pub struct KrivineMachine {
    memory: HashMap<Location, Vec<Term>>,
    environment: HashMap<String, Term>, // always stores closed terms
    expression: Term,
    continuation_stack: Vec<(Choice, Term)>,
    pub steps: u32,
    pub silent: bool,
    output: Vec<String>,
}

fn apply_env(term: Term, env: &HashMap<String, Term>) -> Term {
    match term {
        Term::Variable { ref name } => {
            match env.get(name).cloned() {
                // Recursively resolve in case the stored value is itself a variable
                Some(resolved) => apply_env(resolved, env),
                None => term,
            }
        }
        Term::Choice(_) | Term::Special(_) => term,
        _ => term, // compound terms - free variables will be resolved by `resolve` at use time
    }
}

impl Machine for KrivineMachine {
    fn set_silent(&mut self, to: bool) {
        self.silent = to
    }

    fn print_expression(&self) {
        println!("{}", self.expression)
    }

    fn expand_expression(&mut self) {
        self.expression = self.expression.expand_operations();
    }

    fn output(&self) -> &[String] {
        &self.output
    }

    /// Perform the next transition step.
    fn step(&mut self) -> StepResult {
        self.steps += 1;
        let expr = std::mem::replace(&mut self.expression, Term::id());

        match expr {
            Term::Application {
                function,
                argument,
                location,
            } => {
                // (S_A ; S_a , [N]a.M, K)  →  (S_A ; S_a·N , M, K)

                if location == Location::Output {
                    self.push_output(self.resolve(*argument).to_string());
                } else {
                    self.memory.entry(location).or_default().push(*argument);
                }

                self.expression = *function;

                StepResult::Continue
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                match self.get_from(&location) {
                    Some(argument) => {
                        // close the argument over the current environment
                        // so we never store a term with free variables -> no chains
                        let closed = apply_env(argument, &self.environment);

                        let term = if binds.ends_with("_entry") {
                            term.freshen_locations().freshen_variables()
                        } else {
                            *term
                        };

                        self.environment.insert(binds, closed);
                        self.expression = term;
                        StepResult::Continue
                    }
                    None => StepResult::Failure,
                }
            }
            Term::Variable { name } => match self.environment.get(&name).cloned() {
                Some(term) => {
                    self.expression = term;
                    StepResult::Continue
                }
                None => StepResult::Failure,
            },
            Term::Operation(operation) => {
                self.expression = operation.expand();
                StepResult::Continue
            }
            Term::Choice(choice) => match self.continuation_stack.pop() {
                Some((branch, continuation)) => {
                    if branch == choice {
                        self.expression = continuation;
                    } else {
                        self.expression = Term::Choice(choice);
                    }
                    StepResult::Continue
                }
                None => StepResult::Stop,
            },
            Term::Case { term, exit, then } => {
                self.expression = *term;
                self.continuation_stack.push((exit, *then));
                StepResult::Continue
            }
            Term::Loop { term, branch } => {
                let loop_term = Term::Loop {
                    term: term.clone(),
                    branch: branch.clone(),
                };
                self.continuation_stack.push((branch, loop_term));
                self.expression = *term;
                StepResult::Continue
            }
            Term::Special(special) => match special {
                Special::LogicOr => {
                    if let Some((first, second)) = self.get_two_bools() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first || second,
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::LogicAnd => {
                    if let Some((first, second)) = self.get_two_bools() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first && second,
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::LogicNot => {
                    if let Some(bool) = self.get_bool() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(!bool))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Equal => {
                    if let Some((first, second)) = self.get_two_values() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first == second,
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::NotEqual => {
                    if let Some((first, second)) = self.get_two_values() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first != second,
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::LessThan => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first.as_f32() < second.as_f32(),
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::GreaterThan => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first.as_f32() > second.as_f32(),
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::LessThanEqual => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first.as_f32() <= second.as_f32(),
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::GreaterThanEqual => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Boolean(
                                first.as_f32() >= second.as_f32(),
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Addition => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(number_to_term(first.add(second)));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Subraction => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(number_to_term(first.subtract(second)));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Multiplication => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(number_to_term(first.multiply(second)));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Division => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(number_to_term(first.divide(second)));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::Modulo => {
                    if let Some((first, second)) = self.get_two_nums() {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(number_to_term(first.modulo(second)));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
                Special::IntCast => {
                    if let Some(number) = self
                        .get_from(&Location::Main)
                        .map(|t| self.resolve(t))
                        .and_then(term_to_number)
                    {
                        self.memory
                            .entry(Location::Main)
                            .or_default()
                            .push(Term::Choice(Choice::Constant(Constant::Integer(
                                number.trunc_to_int(),
                            ))));

                        self.expression = Term::Choice(Choice::skip());

                        StepResult::Continue
                    } else {
                        StepResult::Failure
                    }
                }
            },
        }
    }

    /// Perform transition steps until termination.
    ///
    /// Optionally trace the steps.
    fn run(&mut self, trace: bool) -> StepResult {
        if !trace {
            loop {
                let result = self.step();
                if result != StepResult::Continue {
                    return result;
                }
            }
        }

        let mut lines = Vec::new();
        let final_result;

        loop {
            lines.push(self.get_state());
            let result = self.step();
            if result != StepResult::Continue {
                final_result = result;
                break;
            }
        }

        // calculate column widths for trace output
        let mut expression_width = 0;
        let mut cell_column_widths: HashMap<String, usize> = HashMap::new();
        let mut stack_column_widths: HashMap<String, usize> = HashMap::new();
        let mut local_column_widths: HashMap<String, usize> = HashMap::new();

        let mut main_stack_width = 0;
        let mut continuation_stack_width = 0;

        for line in &lines {
            expression_width = expression_width.max(line.expression.chars().count());
            main_stack_width = main_stack_width.max(line.main_stack.chars().count());
            continuation_stack_width =
                continuation_stack_width.max(line.continuation_stack.chars().count());

            for (key, value) in &line.cells {
                let size = value.chars().count();
                let current = *cell_column_widths
                    .get(key)
                    .unwrap_or(&format!("ε{}•0", key).chars().count());

                cell_column_widths.insert(key.clone(), size.max(current));
            }

            for (key, value) in &line.locals {
                let size = value.chars().count();
                let current = *local_column_widths
                    .get(key)
                    .unwrap_or(&format!("ε{}•0", key).chars().count());

                local_column_widths.insert(key.clone(), size.max(current));
            }

            for (key, value) in &line.stacks {
                let size = value.chars().count();
                let current = *stack_column_widths.get(key).unwrap_or(&0);

                if size > current {
                    stack_column_widths.insert(key.clone(), size);
                }
            }
        }

        // output cells and stacks in alpha-numeric order
        let mut cell_order: Vec<&String> = cell_column_widths.keys().collect();
        cell_order.sort();

        let mut locals_order: Vec<&String> = local_column_widths.keys().collect();
        locals_order.sort();

        let mut stack_order: Vec<&String> = stack_column_widths.keys().collect();
        stack_order.sort();

        for line in &lines {
            let mut output_line = "".to_string();

            // display cells first
            for cell in &cell_order {
                output_line.push_str(&format!(
                    " {arg:<width$} ;",
                    arg = line.cells.get(*cell).unwrap_or(&format!("ε{}•0", cell)),
                    width = cell_column_widths.get(*cell).unwrap()
                ));
            }

            // display locals next
            for local in &locals_order {
                output_line.push_str(&format!(
                    " {arg:<width$} ;",
                    arg = line.locals.get(*local).unwrap_or(&format!("ε{}•0", local)),
                    width = local_column_widths.get(*local).unwrap()
                ));
            }

            // display stacks next
            for index in &stack_order {
                output_line.push_str(&format!(
                    " {arg:<width$} ;",
                    arg = line.stacks.get(*index).unwrap_or(&format!("ε{}", index)),
                    width = stack_column_widths.get(*index).unwrap()
                ));
            }

            // display the main stack after all others
            output_line.push_str(&format!(
                " {arg:<width$} ,",
                arg = line.main_stack,
                width = main_stack_width
            ));

            // display the expression
            output_line.push_str(&format!(
                " {arg:>width$} ,",
                arg = line.expression,
                width = expression_width
            ));

            // display the continuation stack
            output_line.push_str(&format!(
                " {arg:>width$}",
                arg = line.continuation_stack,
                width = continuation_stack_width
            ));

            // wrap inside brackets
            output_line = format!("({} )", output_line);

            println!("{}", output_line);
        }

        println!("{}", self.steps);
        final_result
    }
}

impl KrivineMachine {
    pub fn new(expression: Term) -> Self {
        Self {
            memory: HashMap::new(),
            environment: HashMap::new(),
            expression,
            continuation_stack: Vec::new(),
            steps: 0,
            silent: false,
            output: Vec::new(),
        }
    }

    pub fn output_buffer(&self) -> &[String] {
        &self.output
    }

    pub fn seed_input(&mut self, inputs: Vec<Term>) {
        self.memory
            .insert(Location::Input, inputs.into_iter().rev().collect());
    }

    fn push_output(&mut self, line: String) {
        self.output.push(line.clone());
        if !self.silent {
            println!("{}", line);
        }
    }

    /// Pops and returns a term from the given location.
    ///
    /// Note not all location represent stacks:
    ///  - Rnd returns a random Boolean
    ///  - Input gets user input
    ///
    /// Also note cell stacks are have a default value of 0, regular stacks do not.
    fn get_from(&mut self, location: &Location) -> Option<Term> {
        match location {
            Location::Rnd => Some(random_bool_term()),
            Location::RndBool => Some(random_bool_term()),
            Location::RndFloat => Some(random_float_term()),
            Location::Input => self
                .memory
                .entry(Location::Input)
                .or_default()
                .pop()
                .or_else(read_input_term),
            Location::Cell(_) => {
                if self.memory.contains_key(location) {
                    self.memory.entry(location.clone()).or_default().pop()
                } else {
                    self.memory.insert(location.clone(), Vec::new());

                    Some(Term::Choice(Choice::Constant(Constant::Integer(0))))
                }
            }
            Location::Local(_) => {
                if self.memory.contains_key(location) {
                    self.memory.entry(location.clone()).or_default().pop()
                } else {
                    self.memory.insert(location.clone(), Vec::new());

                    Some(Term::Choice(Choice::Constant(Constant::Integer(0))))
                }
            }
            _ => self.memory.entry(location.clone()).or_default().pop(),
        }
    }

    fn resolve(&self, term: Term) -> Term {
        match term {
            Term::Variable { ref name } => {
                match self.environment.get(name).cloned() {
                    Some(resolved) => self.resolve(resolved), // chase chain (safe — stored terms are closed)
                    None => term,
                }
            }
            other => other,
        }
    }

    fn get_bool(&mut self) -> Option<bool> {
        let term = self.get_from(&Location::Main).map(|t| self.resolve(t));
        if let Some(Term::Choice(Choice::Constant(Constant::Boolean(b)))) = term {
            return Some(b);
        }
        None
    }

    fn get_two_values(&mut self) -> Option<(Term, Term)> {
        let second = self.get_from(&Location::Main).map(|t| self.resolve(t));
        let first = self.get_from(&Location::Main).map(|t| self.resolve(t));
        match (first, second) {
            (Some(f), Some(s)) => Some((f, s)),
            _ => None,
        }
    }

    fn get_two_nums(&mut self) -> Option<(Number, Number)> {
        let second = self.get_from(&Location::Main).map(|t| self.resolve(t));
        let first = self.get_from(&Location::Main).map(|t| self.resolve(t));
        match (first, second) {
            (Some(first), Some(second)) => {
                let first = term_to_number(first)?;
                let second = term_to_number(second)?;
                Some((first, second))
            }
            _ => None,
        }
    }

    fn get_two_bools(&mut self) -> Option<(bool, bool)> {
        let second = self.get_from(&Location::Main).map(|t| self.resolve(t));
        let first = self.get_from(&Location::Main).map(|t| self.resolve(t));
        match (first, second) {
            (
                Some(Term::Choice(Choice::Constant(Constant::Boolean(f)))),
                Some(Term::Choice(Choice::Constant(Constant::Boolean(s)))),
            ) => Some((f, s)),
            _ => None,
        }
    }

    // Returns the string representation of the given stack.
    fn stack_to_string(&self, location: &Location) -> String {
        match location {
            Location::Cell(cell) => {
                let start = format!("ε{cell}");

                if let Some(storage) = self.memory.get(location) {
                    storage.iter().fold(start, |acc, x| format!("{acc}•{x}"))
                } else {
                    format!("{start}•0")
                }
            }
            Location::Local(cell) => {
                let start = format!("ε{cell}");

                if let Some(storage) = self.memory.get(location) {
                    storage.iter().fold(start, |acc, x| format!("{acc}•{x}"))
                } else {
                    format!("{start}•0")
                }
            }
            Location::Main => {
                let start = "ελ".to_string();

                if let Some(storage) = self.memory.get(location) {
                    storage.iter().fold(start, |acc, x| format!("{acc}•{x}"))
                } else {
                    start
                }
            }
            Location::Stack(index) => {
                let start = format!("ε{index}");

                if let Some(storage) = self.memory.get(location) {
                    storage.iter().fold(start, |acc, x| format!("{acc}•{x}"))
                } else {
                    start
                }
            }
            _ => {
                panic!()
            }
        }
    }

    /// Gets the current state of the machine as strings. Used for the trace.
    fn get_state(&self) -> State {
        let mut cells = HashMap::new();
        let mut stacks = HashMap::new();
        let mut locals = HashMap::new();
        let mut main_stack = "ελ".to_string();

        for key in self.memory.keys() {
            match key {
                Location::Cell(cell) => {
                    cells.insert(cell.clone(), self.stack_to_string(key));
                }
                Location::Local(cell) => {
                    locals.insert(cell.clone(), self.stack_to_string(key));
                }
                Location::Stack(stack) => {
                    stacks.insert(stack.clone(), self.stack_to_string(key));
                }
                Location::Main => main_stack = self.stack_to_string(key),
                _ => {}
            }
        }

        let expression = self.expression.to_string();

        let mut continuation_stack = "ε".to_string();

        for (branch, term) in self.continuation_stack.clone() {
            continuation_stack = format!("({}→{}){}", branch, term, continuation_stack)
        }

        State {
            cells,
            stacks,
            locals,
            main_stack,
            expression,
            continuation_stack,
        }
    }
}

/// Private type to hold the state of the machine for traces.
struct State {
    cells: HashMap<String, String>,
    locals: HashMap<String, String>,
    stacks: HashMap<String, String>,
    main_stack: String,
    expression: String,
    continuation_stack: String,
}

#[cfg(test)]
// These tests were generated by ChatGPT.
mod tests {
    use super::KrivineMachine;
    use crate::{
        fmc_core::{Choice, Location, Special, Term, choice::Constant},
        machines::{Machine, machine::StepResult},
    };

    #[test]
    fn run_returns_terminal_step_result_and_captures_output() {
        let mut machine = KrivineMachine::new(Term::Application {
            function: Box::new(Term::Choice(Choice::skip())),
            argument: Box::new(Term::Choice(Choice::Constant(Constant::Integer(5)))),
            location: Location::Output,
        });
        machine.set_silent(true);

        let result = machine.run(false);

        assert_eq!(result, StepResult::Stop);
        assert_eq!(machine.output(), ["5"]);
        assert_eq!(machine.output_buffer(), ["5"]);
    }

    #[test]
    fn int_cast_truncates_float_to_zero_towards_zero() {
        let mut machine = KrivineMachine::new(Term::Application {
            function: Box::new(Term::Special(Special::IntCast)),
            argument: Box::new(Term::Choice(Choice::Constant(Constant::Float(-2.75)))),
            location: Location::Main,
        });
        machine.set_silent(true);

        assert_eq!(machine.step(), StepResult::Continue);
        assert_eq!(machine.step(), StepResult::Continue);
        assert_eq!(
            machine.memory.get(&Location::Main),
            Some(&vec![Term::Choice(Choice::Constant(Constant::Integer(-2)))])
        );
        assert_eq!(machine.step(), StepResult::Stop);
    }
}
