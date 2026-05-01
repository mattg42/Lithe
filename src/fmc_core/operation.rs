use std::fmt::Display;

use crate::fmc_core::{
    Choice, Location, Term,
    choice::{Constant, Exception},
    term::fresh_var,
};

#[derive(Debug, Clone, PartialEq)]

/// Encodes constructs for specific effects.
pub enum Operation {
    /// Gets a term from the input stream.
    Read,

    /// Pushes a term to the output stream.
    Write {
        /// Term to output
        argument: Box<Term>,
    },

    /// Updates a memory cell with a term.
    Update {
        /// Cell location to update.
        location: Location,

        /// Term to push to the cell.
        argument: Box<Term>,
    },

    /// Gets a copy of the term at the given cell.
    Lookup {
        cell: String,
    },

    /// Returns either of the provided arguments with equal probabilty.
    Rnd {
        argument1: Box<Term>,
        argument2: Box<Term>,
    },
    // -- Contructs for control flow --
    Sequence {
        first: Box<Term>,
        second: Box<Term>,
    },
    Throw {
        error_code: Choice,
    },
    TryCatch {
        try_: Box<Term>,
        catch: Choice,
        failure: Box<Term>,
    },
    Constant {
        choice: Choice,
    },
    IfThenElse {
        condition: Box<Term>,
        then: Box<Term>,
        else_: Box<Term>,
    },
    Switch {
        term: Box<Term>,
        cases: Vec<(Choice, Box<Term>)>,
    },
    DoWhile {
        term: Box<Term>,
        condition: Box<Term>,
    },
    WhileDo {
        condition: Box<Term>,
        term: Box<Term>,
    },
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Read => write!(f, "read"),
            Operation::Write { argument } => write!(f, "print {argument}"),
            Operation::Update { location, argument } => write!(f, "{location} := {argument}"),
            Operation::Lookup { cell } => write!(f, "${cell}"),
            Operation::Rnd {
                argument1,
                argument2,
            } => write!(f, "{argument1} ⊕ {argument2}"),
            Operation::Sequence { first, second } => write!(f, "{first};{second}"),
            Operation::Throw { error_code } => write!(f, "throw {error_code}"),
            Operation::TryCatch {
                try_,
                catch,
                failure,
            } => write!(f, "try {try_} catch {catch} {failure}"),
            Operation::Constant { choice } => write!(f, "{choice}"),
            Operation::IfThenElse {
                condition,
                then,
                else_,
            } => write!(f, "if {condition} then {then} else {else_}"),
            Operation::Switch { term, cases } => {
                let mut string = format!("case {term} of ");
                let mut iter = cases.iter();
                let (first_case, first_then) = iter.next().unwrap();
                string.push_str(&format!("{first_case} → {first_then}"));

                for (case, then) in iter {
                    string.push_str(&format!(", {case} → {then}"));
                }

                write!(f, "{string}")
            }
            Operation::DoWhile { term, condition } => write!(f, "do {term} while {condition}"),
            Operation::WhileDo { condition, term } => write!(f, "while {condition} do {term}"),
        }
    }
}

impl Operation {
    /// Returns the operation expanded into raw terms.
    pub fn expand(&self) -> Term {
        match self {
            Operation::Read => {
                // read = in⟨x⟩.x
                let new_var = fresh_var();

                Term::Abstraction {
                    binds: new_var.clone(),
                    term: Box::new(Term::Variable { name: new_var }.as_expression()),
                    location: Location::Input,
                }
            }
            Operation::Write { argument } => {
                let new_var = fresh_var();
                Term::Case {
                    term: argument.clone(),
                    exit: Choice::skip(),
                    then: Box::new(Term::Abstraction {
                        binds: new_var.clone(),
                        term: Box::new(Term::Application {
                            function: Box::new(Term::Choice(Choice::skip())),
                            argument: Box::new(Term::Variable { name: new_var }),
                            location: Location::Output,
                        }),
                        location: Location::Main,
                    }),
                }
            }
            Operation::Update { location, argument } =>
            // c := M = M;<x>.c<_>.[x]c.*
            {
                let new_var = fresh_var();
                Term::Case {
                    term: argument.clone(),
                    exit: Choice::skip(),
                    then: Box::new(Term::Abstraction {
                        binds: new_var.clone(),
                        term: Box::new(Term::Abstraction {
                            binds: "_".to_string(),
                            term: Box::new(Term::Application {
                                function: Box::new(Term::Choice(Choice::Exception(
                                    Exception::Skip,
                                ))),
                                argument: Box::new(Term::Variable { name: new_var }),
                                location: location.clone(),
                            }),
                            location: location.clone(),
                        }),
                        location: Location::Main,
                    }),
                }
            }
            Operation::Lookup { cell } => {
                // !c = c⟨x⟩.[x]c.[x].*
                let new_var = fresh_var();
                Term::Abstraction {
                    binds: new_var.clone(),
                    term: Box::new(Term::Application {
                        function: Box::new(Term::Application {
                            function: Box::new(Term::Choice(Choice::skip())),
                            argument: Box::new(Term::Variable {
                                name: new_var.clone(),
                            }),
                            location: Location::Main,
                        }),
                        argument: Box::new(Term::Variable { name: new_var }),
                        location: Location::Cell(cell.clone()),
                    }),
                    location: Location::Cell(cell.clone()),
                }
            }
            Operation::Rnd {
                argument1,
                argument2,
            } => Term::Abstraction {
                // N ⊕ M = rnd⟨x⟩.x M N
                binds: "x".to_string(),
                term: Box::new(Term::Application {
                    function: Box::new(Term::Application {
                        function: Box::new(Term::Variable {
                            name: "x".to_string(),
                        }),
                        argument: argument1.clone(),
                        location: Location::Main,
                    }),
                    argument: argument2.clone(),
                    location: Location::Main,
                }),
                location: Location::Rnd,
            },
            Operation::Sequence { first, second } => Term::Case {
                term: first.clone(),
                exit: Choice::skip(),
                then: second.clone(),
            },
            Operation::Throw { error_code } => Term::Choice(error_code.clone()),
            Operation::TryCatch {
                try_,
                catch,
                failure,
            } => Term::Case {
                term: try_.clone(),
                exit: catch.clone(),
                then: failure.clone(),
            },
            Operation::Constant { choice } => Term::Application {
                function: Box::new(Term::Choice(Choice::skip())),
                argument: Box::new(Term::Choice(choice.clone())),
                location: Location::Main,
            },
            Operation::IfThenElse {
                condition,
                then,
                else_,
            } => Term::Case {
                term: Box::new(Term::Case {
                    term: Box::new(
                        Operation::Sequence {
                            first: condition.clone(),
                            second: Box::new(Term::id()),
                        }
                        .expand(),
                    ),

                    exit: Choice::Constant(Constant::Boolean(true)),
                    then: then.clone(),
                }),
                exit: Choice::Constant(Constant::Boolean(false)),
                then: else_.clone(),
            },
            Operation::Switch { term, cases } => {
                let base_term = Operation::Sequence {
                    first: term.clone(),
                    second: Box::new(Term::id()),
                }
                .expand();

                cases.iter().fold(base_term, |acc, x| Term::Case {
                    term: Box::new(acc),
                    exit: x.0.clone(),
                    then: x.1.clone(),
                })
            }
            Operation::DoWhile { term, condition } => Term::Case {
                term: Box::new(Term::Case {
                    term: Box::new(Term::Loop {
                        term: Box::new(
                            Operation::Sequence {
                                first: Box::new(
                                    Operation::Sequence {
                                        first: term.clone(),
                                        second: condition.clone(),
                                    }
                                    .expand(),
                                ),
                                second: Box::new(Term::id()),
                            }
                            .expand(),
                        ),
                        branch: Choice::Constant(Constant::Boolean(true)),
                    }),
                    exit: Choice::Constant(Constant::Boolean(false)),
                    then: Box::new(Term::Choice(Choice::skip())),
                }),
                exit: Choice::Exception(Exception::Break),
                then: Box::new(Term::Choice(Choice::skip())),
            },
            Operation::WhileDo { condition, term } => Term::Case {
                // (B ; ⟨x⟩.x ; ⊤→M)⋆ ; ⊥→⋆ ; break→⋆
                //
                term: Box::new(Term::Case {
                    term: Box::new(Term::Loop {
                        term: Box::new(Term::Case {
                            term: Box::new(Term::Case {
                                term: condition.clone(),
                                exit: Choice::skip(),
                                then: Box::new(Term::id()),
                            }),
                            exit: Choice::Constant(Constant::Boolean(true)),
                            then: term.clone(),
                        }),
                        branch: Choice::skip(),
                    }),
                    exit: Choice::Constant(Constant::Boolean(false)),
                    then: Box::new(Term::Choice(Choice::skip())),
                }),
                exit: Choice::Exception(Exception::Break),
                then: Box::new(Term::Choice(Choice::skip())),
            },
        }
    }
}

#[cfg(test)]
// These tests were generated by ChatGPT.
mod tests {
    use super::Operation;
    use crate::fmc_core::{
        Choice, Location, Term,
        choice::{Constant, Exception},
    };

    fn v(name: &str) -> Term {
        Term::Variable {
            name: name.to_string(),
        }
    }

    fn int(value: i32) -> Term {
        Term::Choice(Choice::Constant(Constant::Integer(value)))
    }

    fn bool_choice(value: bool) -> Choice {
        Choice::Constant(Constant::Boolean(value))
    }

    #[test]
    fn expands_read_to_input_identity_abstraction() {
        let expanded = Operation::Read.expand();

        match expanded {
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                assert_eq!(location, Location::Input);
                assert_eq!(*term, v(&binds).as_expression());
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_write_to_case_followed_by_output_application() {
        let argument = v("x");
        let expanded = Operation::Write {
            argument: Box::new(argument.clone()),
        }
        .expand();

        match expanded {
            Term::Case { term, exit, then } => {
                assert_eq!(*term, argument);
                assert_eq!(exit, Choice::skip());

                match *then {
                    Term::Abstraction {
                        binds,
                        term,
                        location,
                    } => {
                        assert_eq!(location, Location::Main);
                        assert_eq!(
                            *term,
                            Term::Application {
                                function: Box::new(Term::Choice(Choice::skip())),
                                argument: Box::new(v(&binds)),
                                location: Location::Output,
                            }
                        );
                    }
                    other => panic!("unexpected continuation: {other:?}"),
                }
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_update_to_case_with_nested_abstractions() {
        let argument = int(5);
        let location = Location::Cell("c1".to_string());
        let expanded = Operation::Update {
            location: location.clone(),
            argument: Box::new(argument.clone()),
        }
        .expand();

        match expanded {
            Term::Case { term, exit, then } => {
                assert_eq!(*term, argument);
                assert_eq!(exit, Choice::skip());

                match *then {
                    Term::Abstraction {
                        binds,
                        term,
                        location: outer_location,
                    } => {
                        assert_eq!(outer_location, Location::Main);

                        match *term {
                            Term::Abstraction {
                                binds: inner_binds,
                                term,
                                location: inner_location,
                            } => {
                                assert_eq!(inner_binds, "_");
                                assert_eq!(inner_location, location);
                                assert_eq!(
                                    *term,
                                    Term::Application {
                                        function: Box::new(Term::Choice(Choice::skip())),
                                        argument: Box::new(v(&binds)),
                                        location: location.clone(),
                                    }
                                );
                            }
                            other => panic!("unexpected inner abstraction: {other:?}"),
                        }
                    }
                    other => panic!("unexpected continuation: {other:?}"),
                }
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_lookup_to_cell_abstraction_reusing_the_lookup_value() {
        let expanded = Operation::Lookup {
            cell: "c1".to_string(),
        }
        .expand();

        match expanded {
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                assert_eq!(location, Location::Cell("c1".to_string()));
                assert_eq!(
                    *term,
                    Term::Application {
                        function: Box::new(Term::Application {
                            function: Box::new(Term::Choice(Choice::skip())),
                            argument: Box::new(v(&binds)),
                            location: Location::Main,
                        }),
                        argument: Box::new(v(&binds)),
                        location: Location::Cell("c1".to_string()),
                    }
                );
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_rnd_to_rnd_abstraction() {
        let expanded = Operation::Rnd {
            argument1: Box::new(v("left")),
            argument2: Box::new(v("right")),
        }
        .expand();

        assert_eq!(
            expanded,
            Term::Abstraction {
                binds: "x".to_string(),
                term: Box::new(Term::Application {
                    function: Box::new(Term::Application {
                        function: Box::new(v("x")),
                        argument: Box::new(v("left")),
                        location: Location::Main,
                    }),
                    argument: Box::new(v("right")),
                    location: Location::Main,
                }),
                location: Location::Rnd,
            }
        );
    }

    #[test]
    fn expands_simple_control_flow_variants() {
        let first = v("first");
        let second = v("second");
        assert_eq!(
            Operation::Sequence {
                first: Box::new(first.clone()),
                second: Box::new(second.clone()),
            }
            .expand(),
            Term::Case {
                term: Box::new(first),
                exit: Choice::skip(),
                then: Box::new(second),
            }
        );

        assert_eq!(
            Operation::Throw {
                error_code: Choice::Exception(Exception::Return),
            }
            .expand(),
            Term::Choice(Choice::Exception(Exception::Return))
        );

        assert_eq!(
            Operation::TryCatch {
                try_: Box::new(v("try_body")),
                catch: Choice::Exception(Exception::Exception("err".to_string())),
                failure: Box::new(v("recover")),
            }
            .expand(),
            Term::Case {
                term: Box::new(v("try_body")),
                exit: Choice::Exception(Exception::Exception("err".to_string())),
                then: Box::new(v("recover")),
            }
        );

        assert_eq!(
            Operation::Constant {
                choice: Choice::Constant(Constant::Integer(9)),
            }
            .expand(),
            Term::Application {
                function: Box::new(Term::Choice(Choice::skip())),
                argument: Box::new(Term::Choice(Choice::Constant(Constant::Integer(9)))),
                location: Location::Main,
            }
        );
    }

    #[test]
    fn expands_if_then_else_to_nested_cases() {
        let condition = v("cond");
        let then_branch = v("then_branch");
        let else_branch = v("else_branch");

        let expanded = Operation::IfThenElse {
            condition: Box::new(condition.clone()),
            then: Box::new(then_branch.clone()),
            else_: Box::new(else_branch.clone()),
        }
        .expand();

        match expanded {
            Term::Case { term, exit, then } => {
                assert_eq!(exit, bool_choice(false));
                assert_eq!(*then, else_branch);

                match *term {
                    Term::Case {
                        term: inner_term,
                        exit: inner_exit,
                        then: inner_then,
                    } => {
                        assert_eq!(inner_exit, bool_choice(true));
                        assert_eq!(*inner_then, then_branch);

                        match *inner_term {
                            Term::Case {
                                term: seq_term,
                                exit: seq_exit,
                                then: seq_then,
                            } => {
                                assert_eq!(*seq_term, condition);
                                assert_eq!(seq_exit, Choice::skip());

                                match *seq_then {
                                    Term::Abstraction {
                                        binds,
                                        term,
                                        location,
                                    } => {
                                        assert_eq!(location, Location::Main);
                                        assert_eq!(*term, v(&binds));
                                    }
                                    other => panic!("unexpected identity term: {other:?}"),
                                }
                            }
                            other => panic!("unexpected sequence expansion: {other:?}"),
                        }
                    }
                    other => panic!("unexpected inner case: {other:?}"),
                }
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_switch_to_folded_cases() {
        let expanded = Operation::Switch {
            term: Box::new(v("subject")),
            cases: vec![
                (Choice::Constant(Constant::Integer(1)), Box::new(v("one"))),
                (Choice::Constant(Constant::Integer(2)), Box::new(v("two"))),
            ],
        }
        .expand();

        match expanded {
            Term::Case { term, exit, then } => {
                assert_eq!(exit, Choice::Constant(Constant::Integer(2)));
                assert_eq!(*then, v("two"));

                match *term {
                    Term::Case {
                        term: inner_term,
                        exit: inner_exit,
                        then: inner_then,
                    } => {
                        assert_eq!(inner_exit, Choice::Constant(Constant::Integer(1)));
                        assert_eq!(*inner_then, v("one"));

                        match *inner_term {
                            Term::Case {
                                term: seq_term,
                                exit: seq_exit,
                                then: seq_then,
                            } => {
                                assert_eq!(*seq_term, v("subject"));
                                assert_eq!(seq_exit, Choice::skip());

                                match *seq_then {
                                    Term::Abstraction {
                                        binds,
                                        term,
                                        location,
                                    } => {
                                        assert_eq!(location, Location::Main);
                                        assert_eq!(*term, v(&binds));
                                    }
                                    other => panic!("unexpected identity term: {other:?}"),
                                }
                            }
                            other => panic!("unexpected sequence expansion: {other:?}"),
                        }
                    }
                    other => panic!("unexpected inner case: {other:?}"),
                }
            }
            other => panic!("unexpected expansion: {other:?}"),
        }
    }

    #[test]
    fn expands_do_while_and_while_do_with_break_handling() {
        let do_while = Operation::DoWhile {
            term: Box::new(v("body")),
            condition: Box::new(v("cond")),
        }
        .expand();
        let while_do = Operation::WhileDo {
            condition: Box::new(v("cond")),
            term: Box::new(v("body")),
        }
        .expand();

        match do_while {
            Term::Case { exit, then, .. } => {
                assert_eq!(exit, Choice::Exception(Exception::Break));
                assert_eq!(*then, Term::Choice(Choice::skip()));
            }
            other => panic!("unexpected do-while expansion: {other:?}"),
        }

        match while_do {
            Term::Case { exit, then, .. } => {
                assert_eq!(exit, Choice::Exception(Exception::Break));
                assert_eq!(*then, Term::Choice(Choice::skip()));
            }
            other => panic!("unexpected while-do expansion: {other:?}"),
        }
    }
}
