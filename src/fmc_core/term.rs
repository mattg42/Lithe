use std::{
    collections::HashSet,
    fmt::{Display, Error, Formatter},
};

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::fmc_core::{Choice, Location, Operation, Special};

static FRESH_VAR_COUNTER: AtomicUsize = AtomicUsize::new(1);
/// Global function to get a fresh variable.
///
/// For example, used when performing capture-avoiding substitution.
pub fn fresh_var() -> String {
    let id = FRESH_VAR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("x{}", id)
}

static FRESH_LOCATION_COUNTER: AtomicUsize = AtomicUsize::new(1);
/// Global function to get a fresh location.
pub fn fresh_location() -> String {
    let id = FRESH_LOCATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("c{}", id)
}

#[derive(Clone, PartialEq, Debug)]
/// Recursively defines a term of the calculus.
pub enum Term {
    Variable {
        name: String,
    },
    Application {
        function: Box<Term>,
        argument: Box<Term>,
        location: Location,
    },
    Abstraction {
        binds: String,
        term: Box<Term>,
        location: Location,
    },
    Operation(Operation),
    Choice(Choice),
    Case {
        term: Box<Term>,
        exit: Choice,
        then: Box<Term>,
    },
    Loop {
        term: Box<Term>,
        branch: Choice,
    },
    Special(Special),
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_prec(f, 0)
    }
}

impl Term {
    fn fmt_prec(&self, f: &mut Formatter<'_>, prec: u8) -> Result<(), Error> {
        match self {
            Term::Case { term, exit, then } => {
                let my_prec = 1;
                let need_parens = prec > my_prec;

                if need_parens {
                    write!(f, "(")?;
                }

                term.fmt_prec(f, my_prec + 1)?;
                write!(f, ";")?;
                if *exit != Choice::skip() {
                    write!(f, "{}->", exit)?;
                }
                then.fmt_prec(f, my_prec)?;

                if need_parens {
                    write!(f, ")")?;
                }

                Ok(())
            }

            Term::Loop { term, branch } => {
                let my_prec = 2;
                let need_parens = prec > my_prec;

                if need_parens {
                    write!(f, "(")?;
                }

                term.fmt_prec(f, my_prec)?;
                write!(f, "^{}", branch)?;

                if need_parens {
                    write!(f, ")")?;
                }

                Ok(())
            }

            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                if *location == Location::Main {
                    write!(f, "<{}>.", binds)?;
                } else {
                    write!(f, "{}<{}>.", location, binds)?;
                }
                term.fmt_prec(f, 2)
            }

            Term::Application {
                function,
                argument,
                location,
            } => {
                write!(f, "[")?;
                argument.fmt_prec(f, 0)?;
                write!(f, "]")?;
                if *location != Location::Main {
                    write!(f, "{}", location)?;
                }
                write!(f, ".")?;
                function.fmt_prec(f, 2)
            }

            Term::Variable { name } => write!(f, "{}", name),
            Term::Choice(choice) => write!(f, "{}", choice),
            Term::Special(special) => write!(f, "{}", special),
            Term::Operation(operation) => write!(f, "{}", operation),
        }
    }
}

impl Term {
    /// Recursively expands all the operation terms to their raw forms.
    pub fn expand_operations(&self) -> Term {
        match self {
            Term::Variable { .. } => self.clone(),
            Term::Abstraction {
                binds,
                term,
                location,
            } => Term::Abstraction {
                binds: binds.clone(),
                term: Box::new(term.expand_operations()),
                location: location.clone(),
            },
            Term::Application {
                function,
                argument,
                location,
            } => Term::Application {
                function: Box::new(function.expand_operations()),
                argument: Box::new(argument.expand_operations()),
                location: location.clone(),
            },
            Term::Operation(operation) => operation.expand().expand_operations(),
            Term::Choice(_) => self.clone(),
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.expand_operations()),
                exit: exit.clone(),
                then: Box::new(then.expand_operations()),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.expand_operations()),
                branch: branch.clone(),
            },
            Term::Special(_) => self.clone(),
        }
    }

    /// Computes the free variables in a term
    pub fn free_variables(&self) -> HashSet<String> {
        let mut all_frees: HashSet<String> = HashSet::new();

        fn recurse(term: Term, free: &mut HashSet<String>) {
            match term {
                Term::Variable { name } => {
                    free.insert(name);
                }
                Term::Abstraction { binds, term, .. } => {
                    recurse(*term, free);
                    free.remove(&binds);
                }
                Term::Application {
                    function, argument, ..
                } => {
                    recurse(*function, free);
                    recurse(*argument, free);
                }
                Term::Operation(_) => {
                    panic!("operation found")
                }
                Term::Choice(_) => {}
                Term::Case { term, then, .. } => {
                    recurse(*term, free);
                    recurse(*then, free);
                }
                Term::Loop { term, .. } => {
                    recurse(*term, free);
                }
                Term::Special(_) => {}
            }
        }

        recurse(self.clone(), &mut all_frees);

        all_frees
    }

    pub fn get_bound_variables(&self) -> HashSet<String> {
        let mut bound = HashSet::new();

        fn recurse(term: &Term, bound: &mut HashSet<String>) {
            match term {
                Term::Abstraction { binds, term, .. } => {
                    bound.insert(binds.clone());
                    recurse(term, bound);
                }
                Term::Application {
                    function, argument, ..
                } => {
                    recurse(function, bound);
                    recurse(argument, bound);
                }
                Term::Case { term, then, .. } => {
                    recurse(term, bound);
                    recurse(then, bound);
                }
                Term::Loop { term, .. } => recurse(term, bound),
                _ => {}
            }
        }

        recurse(self, &mut bound);
        bound
    }

    pub fn rename_variable(&self, from: &str, to: &str) -> Term {
        match self {
            Term::Variable { name } => {
                if name == from {
                    Term::Variable {
                        name: to.to_string(),
                    }
                } else {
                    self.clone()
                }
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                if binds == from {
                    Term::Abstraction {
                        binds: to.to_string(),
                        term: Box::new(term.rename_variable(from, to)),
                        location: location.clone(),
                    }
                } else {
                    Term::Abstraction {
                        binds: binds.clone(),
                        term: Box::new(term.rename_variable(from, to)),
                        location: location.clone(),
                    }
                }
            }
            Term::Application {
                function,
                argument,
                location,
            } => Term::Application {
                function: Box::new(function.rename_variable(from, to)),
                argument: Box::new(argument.rename_variable(from, to)),
                location: location.clone(),
            },
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.rename_variable(from, to)),
                exit: exit.clone(),
                then: Box::new(then.rename_variable(from, to)),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.rename_variable(from, to)),
                branch: branch.clone(),
            },
            _ => self.clone(),
        }
    }

    pub fn freshen_variables(&self) -> Term {
        let bound = self.get_bound_variables();
        let mut current = self.clone();
        for var in bound {
            if var.ends_with("_entry") {
                current = current.rename_variable(&var, &format!("{}_entry", fresh_var()));
            } else {
                current = current.rename_variable(&var, &fresh_var());
            }
        }
        current
    }

    /// Computes the bound location in a term
    pub fn get_all_locations(&self) -> HashSet<Location> {
        let mut all_locations: HashSet<Location> = HashSet::new();

        fn recurse(term: Term, locations: &mut HashSet<Location>) {
            match term {
                Term::Abstraction { term, location, .. } => {
                    locations.insert(location);
                    recurse(*term, locations);
                }
                Term::Application {
                    function,
                    argument,
                    location,
                } => {
                    locations.insert(location);
                    recurse(*function, locations);
                    recurse(*argument, locations);
                }
                Term::Case { term, then, .. } => {
                    recurse(*term, locations);
                    recurse(*then, locations);
                }
                Term::Loop { term, .. } => {
                    recurse(*term, locations);
                }
                _ => {}
            }
        }

        recurse(self.clone(), &mut all_locations);

        all_locations
    }

    pub fn freshen_locations(&self) -> Term {
        let locations = self.get_all_locations();
        let mut current_term = self.clone();
        for loc in locations {
            current_term = match loc {
                Location::Local(cell) => current_term.rename_locals(cell, fresh_location()),
                _ => current_term,
            };
        }

        current_term
    }

    // Perform capture-avoiding substitution.
    pub fn substitute(&self, variable: &str, with: Term) -> Term {
        match self {
            Term::Variable { name } => {
                if name == variable {
                    with
                } else {
                    self.clone()
                }
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                if binds == variable {
                    self.clone()
                } else if with.free_variables().contains(binds) {
                    // Rename binding var and bound occurences.
                    let new_binds = fresh_var();

                    // rename binding var (alpha conversion).
                    let capture_avoiding = term.substitute(
                        binds,
                        Term::Variable {
                            name: new_binds.clone(),
                        },
                    );

                    Term::Abstraction {
                        binds: new_binds,
                        term: Box::new(capture_avoiding.substitute(variable, with)),
                        location: location.clone(),
                    }
                } else {
                    Term::Abstraction {
                        binds: binds.clone(),
                        term: Box::new(term.substitute(variable, with)),
                        location: location.clone(),
                    }
                }
            }
            Term::Application {
                function,
                argument,
                location,
            } => Term::Application {
                function: Box::new(function.substitute(variable, with.clone())),
                argument: Box::new(argument.substitute(variable, with)),
                location: location.clone(),
            },
            Term::Operation(_) => {
                panic!("operation found")
            }
            Term::Choice(_) => self.clone(),
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.substitute(variable, with.clone())),
                exit: exit.clone(),
                then: Box::new(then.substitute(variable, with)),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.substitute(variable, with)),
                branch: branch.clone(),
            },
            Term::Special(_) => self.clone(),
        }
    }

    /// Returns the beta reduction of the current term.
    fn beta_reduce(&self) -> Option<Term> {
        match self {
            Term::Application {
                function,
                argument,
                location,
            } => {
                let app_location = location;

                match function.as_ref() {
                    Term::Abstraction {
                        binds,
                        term,
                        location,
                    } => {
                        if app_location == location {
                            Some(term.substitute(binds.as_str(), *argument.clone()))
                        } else {
                            let free_vars = argument.free_variables();
                            let new_binds = if free_vars.contains(binds) {
                                fresh_var()
                            } else {
                                binds.clone()
                            };

                            let new_term = if free_vars.contains(binds) {
                                Box::new(term.substitute(
                                    binds,
                                    Term::Variable {
                                        name: new_binds.clone(),
                                    },
                                ))
                            } else {
                                term.clone()
                            };

                            Some(Term::Abstraction {
                                binds: new_binds,
                                term: Box::new(Term::Application {
                                    function: new_term,
                                    argument: argument.clone(),
                                    location: app_location.clone(),
                                }),
                                location: location.clone(),
                            })
                        }
                    }
                    _ => None,
                }
            }
            Term::Variable { .. } => None,
            Term::Abstraction { .. } => None,
            Term::Operation(_) => None,
            Term::Choice(_) => None,
            Term::Case { term, exit, then } => {
                let exit1 = exit.clone();
                let then1 = then.clone();
                match term.as_ref() {
                    Term::Choice(choice) => {
                        if choice == exit {
                            Some(*then.clone())
                        } else {
                            Some(*term.clone())
                        }
                    }
                    Term::Abstraction {
                        binds,
                        term,
                        location,
                    } => {
                        let free_vars = then.free_variables();
                        let new_binds = if free_vars.contains(binds) {
                            fresh_var()
                        } else {
                            binds.clone()
                        };

                        let new_term = if free_vars.contains(binds) {
                            Box::new(term.substitute(
                                binds,
                                Term::Variable {
                                    name: new_binds.clone(),
                                },
                            ))
                        } else {
                            term.clone()
                        };

                        Some(Term::Abstraction {
                            binds: new_binds,
                            term: Box::new(Term::Case {
                                term: new_term,
                                exit: exit.clone(),
                                then: then.clone(),
                            }),
                            location: location.clone(),
                        })
                    }
                    Term::Application {
                        function,
                        argument,
                        location,
                    } => Some(Term::Application {
                        function: Box::new(Term::Case {
                            term: function.clone(),
                            exit: exit.clone(),
                            then: then.clone(),
                        }),
                        argument: argument.clone(),
                        location: location.clone(),
                    }),
                    Term::Case { term, exit, then } => {
                        if *exit == exit1 {
                            Some(Term::Case {
                                term: term.clone(),
                                exit: exit.clone(),
                                then: Box::new(Term::Case {
                                    term: then.clone(),
                                    exit: exit.clone(),
                                    then: then1,
                                }),
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            Term::Loop { term, branch } => Some(Term::Case {
                term: term.clone(),
                exit: branch.clone(),
                then: Box::new(Term::Loop {
                    term: term.clone(),
                    branch: branch.clone(),
                }),
            }),
            Term::Special(_) => panic!(),
        }
    }

    fn fmc_normal_reduce(&self, count: u32) -> Term {
        if let Some(reduced) = self.compute_reduce() {
            reduced
        } else if let Some(reduced) = self.structure_reduce() {
            reduced
        } else if let Some(reduced) = self.descend_reduce(count) {
            reduced
        // } else if let Some(reduced) = self.expand_reduce(count) {
        //     reduced
        } else {
            self.clone()
        }
    }

    #[allow(dead_code)]
    fn expand_reduce(&self, count: u32) -> Option<Term> {
        if count > 2 {
            None
        } else {
            match self {
                Term::Loop { term, branch } => {
                    let unrolled = Term::Case {
                        term: term.clone(),
                        exit: branch.clone(),
                        then: Box::new(self.clone()), // reinstates the loop
                    };

                    Some(unrolled.fmc_normal_reduce(count + 1))
                }
                _ => None,
            }
        }
    }
    fn descend_reduce(&self, count: u32) -> Option<Term> {
        match self {
            // Application: function first, then argument
            Term::Application {
                function,
                argument,
                location,
            } => {
                // left
                let f2 = function.fmc_normal_reduce(count);
                if f2 != **function {
                    return Some(Term::Application {
                        function: Box::new(f2),
                        argument: argument.clone(),
                        location: location.clone(),
                    });
                }

                // right
                let a2 = argument.fmc_normal_reduce(count);
                if a2 != **argument {
                    return Some(Term::Application {
                        function: function.clone(),
                        argument: Box::new(a2),
                        location: location.clone(),
                    });
                }

                None
            }

            // Abstraction: reduce body
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                let t2 = term.fmc_normal_reduce(count);
                if t2 != **term {
                    return Some(Term::Abstraction {
                        binds: binds.clone(),
                        term: Box::new(t2),
                        location: location.clone(),
                    });
                }

                None
            }

            // Case: M ; i → K
            Term::Case { term, exit, then } => {
                // reduce M first
                let m2 = term.fmc_normal_reduce(count);
                if m2 != **term {
                    return Some(Term::Case {
                        term: Box::new(m2),
                        exit: exit.clone(),
                        then: then.clone(),
                    });
                }

                // then continuation
                let k2 = then.fmc_normal_reduce(count);
                if k2 != **then {
                    return Some(Term::Case {
                        term: term.clone(),
                        exit: exit.clone(),
                        then: Box::new(k2),
                    });
                }

                None
            }

            // Loop: only reduce inside, DO NOT unroll
            Term::Loop { term, branch } => {
                let m2 = term.fmc_normal_reduce(count);
                if m2 != **term {
                    return Some(Term::Loop {
                        term: Box::new(m2),
                        branch: branch.clone(),
                    });
                }

                None
            }

            // Everything else: no descent
            Term::Variable { .. } | Term::Operation(_) | Term::Choice(_) | Term::Special(_) => None,
        }
    }

    fn compute_reduce(&self) -> Option<Term> {
        match self {
            Term::Case { term, exit, then } => match term.as_ref() {
                Term::Choice(choice) => {
                    if choice == exit {
                        Some(*then.clone())
                    } else {
                        Some(*term.clone())
                    }
                }
                _ => None,
            },
            Term::Application {
                function,
                argument,
                location,
            } => {
                let app_location = location;

                match function.as_ref() {
                    Term::Abstraction {
                        binds,
                        term,
                        location,
                    } => {
                        if app_location == location {
                            Some(term.substitute(binds.as_str(), *argument.clone()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn structure_reduce(&self) -> Option<Term> {
        match self {
            Term::Case { term, exit, then } => match term.as_ref() {
                // Prefix (pop)
                Term::Abstraction {
                    binds,
                    term: inner_term,
                    location,
                } => {
                    let free_vars = then.free_variables();
                    let new_binds = if free_vars.contains(binds) {
                        fresh_var()
                    } else {
                        binds.clone()
                    };

                    let new_then = if free_vars.contains(binds) {
                        Box::new(then.substitute(
                            binds,
                            Term::Variable {
                                name: new_binds.clone(),
                            },
                        ))
                    } else {
                        then.clone()
                    };

                    Some(Term::Abstraction {
                        binds: new_binds,
                        term: Box::new(Term::Case {
                            term: inner_term.clone(),
                            exit: exit.clone(),
                            then: new_then,
                        }),
                        location: location.clone(),
                    })
                }
                // Prefix (push)
                Term::Application {
                    function,
                    argument,
                    location,
                } => Some(Term::Application {
                    function: Box::new(Term::Case {
                        term: function.clone(),
                        exit: exit.clone(),
                        then: then.clone(),
                    }),
                    argument: argument.clone(),
                    location: location.clone(),
                }),
                // Associate
                Term::Case {
                    term: inner_term,
                    exit: inner_exit,
                    then: inner_then,
                } => {
                    if exit == inner_exit {
                        Some(Term::Case {
                            term: inner_term.clone(),
                            exit: exit.clone(),
                            then: Box::new(Term::Case {
                                term: inner_then.clone(),
                                exit: exit.clone(),
                                then: then.clone(),
                            }),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Term::Application {
                function,
                argument,
                location,
            } => {
                let app_location = location;

                match function.as_ref() {
                    Term::Abstraction {
                        binds,
                        term,
                        location,
                    } => {
                        if app_location != location {
                            let free_vars = argument.free_variables();
                            let new_binds = if free_vars.contains(binds) {
                                fresh_var()
                            } else {
                                binds.clone()
                            };

                            let new_term = if free_vars.contains(binds) {
                                Box::new(term.substitute(
                                    binds,
                                    Term::Variable {
                                        name: new_binds.clone(),
                                    },
                                ))
                            } else {
                                term.clone()
                            };

                            Some(Term::Abstraction {
                                binds: new_binds,
                                term: Box::new(Term::Application {
                                    function: new_term,
                                    argument: argument.clone(),
                                    location: app_location.clone(),
                                }),
                                location: location.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    /// Finds and reduces the next redex using weak head reduction.
    fn weak_head_reduce(&self) -> Term {
        if let Some(reduced) = self.beta_reduce() {
            reduced
        } else {
            match self {
                Term::Application {
                    function,
                    argument,
                    location,
                } => Term::Application {
                    function: Box::new(function.weak_head_reduce()),
                    argument: argument.clone(),
                    location: location.clone(),
                },
                _ => self.clone(),
            }
        }
    }

    #[allow(dead_code)]
    /// Finds and reduces the next redex using normal order reduction.
    fn normal_order(&self) -> Term {
        if let Some(reduced) = self.beta_reduce() {
            reduced
        } else {
            match self {
                Term::Variable { .. } => self.clone(),
                Term::Application {
                    function,
                    argument,
                    location,
                } => {
                    let new_function = function.normal_order();
                    if new_function == **function {
                        let new_argument = argument.normal_order();
                        Term::Application {
                            function: function.clone(),
                            argument: Box::new(new_argument),
                            location: location.clone(),
                        }
                    } else {
                        Term::Application {
                            function: Box::new(new_function.clone()),
                            argument: argument.clone(),
                            location: location.clone(),
                        }
                    }
                }
                Term::Abstraction {
                    binds,
                    term,
                    location,
                } => Term::Abstraction {
                    binds: binds.clone(),
                    term: Box::new(term.clone().normal_order()),
                    location: location.clone(),
                },
                Term::Operation(_) => self.clone(),
                Term::Choice(_) => self.clone(),
                Term::Case { term, exit, then } => {
                    let new_term = term.normal_order();
                    if new_term == **term {
                        let new_then = then.normal_order();
                        Term::Case {
                            term: term.clone(),
                            exit: exit.clone(),
                            then: Box::new(new_then),
                        }
                    } else {
                        Term::Case {
                            term: Box::new(new_term),
                            exit: exit.clone(),
                            then: then.clone(),
                        }
                    }
                }
                _ => self.clone(),
            }
        }
    }

    pub fn rename_cells(&self, from: String, to: String) -> Term {
        match self {
            Term::Application {
                function,
                argument,
                location,
            } => {
                let new_location = if let Location::Cell(cell) = location.clone() {
                    if cell == from {
                        &Location::Cell(to.clone())
                    } else {
                        location
                    }
                } else {
                    location
                };

                Term::Application {
                    function: Box::new(function.rename_cells(from.clone(), to.clone())),
                    argument: Box::new(argument.rename_cells(from, to)),
                    location: new_location.clone(),
                }
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                let new_location = if let Location::Cell(cell) = location.clone() {
                    if cell == from {
                        &Location::Cell(to.clone())
                    } else {
                        location
                    }
                } else {
                    location
                };

                Term::Abstraction {
                    binds: binds.to_string(),
                    term: Box::new(term.rename_cells(from, to)),
                    location: new_location.clone(),
                }
            }
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.rename_cells(from.clone(), to.clone())),
                exit: exit.clone(),
                then: Box::new(then.rename_cells(from.clone(), to.clone())),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.rename_cells(from.clone(), to.clone())),
                branch: branch.clone(),
            },
            _ => self.clone(),
        }
    }

    pub fn switch_cell_to_local(&self) -> Term {
        match self {
            Term::Application {
                function,
                argument,
                location,
            } => {
                let new_location = if let Location::Cell(cell) = location.clone() {
                    &Location::Local(cell.clone())
                } else {
                    location
                };

                Term::Application {
                    function: Box::new(function.switch_cell_to_local()),
                    argument: Box::new(argument.switch_cell_to_local()),
                    location: new_location.clone(),
                }
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                let new_location = if let Location::Cell(cell) = location.clone() {
                    &Location::Local(cell.clone())
                } else {
                    location
                };

                Term::Abstraction {
                    binds: binds.to_string(),
                    term: Box::new(term.switch_cell_to_local()),
                    location: new_location.clone(),
                }
            }
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.switch_cell_to_local()),
                exit: exit.clone(),
                then: Box::new(then.switch_cell_to_local()),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.switch_cell_to_local()),
                branch: branch.clone(),
            },
            _ => self.clone(),
        }
    }

    pub fn rename_locals(&self, from: String, to: String) -> Term {
        match self {
            Term::Application {
                function,
                argument,
                location,
            } => {
                let new_location = if let Location::Local(cell) = location.clone() {
                    if cell == from {
                        &Location::Local(to.clone())
                    } else {
                        location
                    }
                } else {
                    location
                };

                Term::Application {
                    function: Box::new(function.rename_locals(from.clone(), to.clone())),
                    argument: Box::new(argument.rename_locals(from, to)),
                    location: new_location.clone(),
                }
            }
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                let new_location = if let Location::Local(cell) = location.clone() {
                    if cell == from {
                        &Location::Local(to.clone())
                    } else {
                        location
                    }
                } else {
                    location
                };

                Term::Abstraction {
                    binds: binds.to_string(),
                    term: Box::new(term.rename_locals(from, to)),
                    location: new_location.clone(),
                }
            }
            Term::Case { term, exit, then } => Term::Case {
                term: Box::new(term.rename_locals(from.clone(), to.clone())),
                exit: exit.clone(),
                then: Box::new(then.rename_locals(from.clone(), to.clone())),
            },
            Term::Loop { term, branch } => Term::Loop {
                term: Box::new(term.rename_locals(from.clone(), to.clone())),
                branch: branch.clone(),
            },
            _ => self.clone(),
        }
    }

    /// Beta-reduces to normal form, using the specified reduction order.
    #[allow(unused_variables)]
    pub fn compute_reduction(&self, weak_head: bool) -> Term {
        let mut before = self.clone();
        // if weak_head {
        //     while before != before.weak_head_reduce() {
        //         before = before.weak_head_reduce()
        //     }
        // } else {
        //     while before != before.any_reduce() {
        //         before = before.any_reduce()
        //     }
        // }
        let mut count = 0;
        loop {
            let next = before.fmc_normal_reduce(0);
            // println!("{}", next);
            if next == before {
                break;
            }
            before = next;
            count += 1;
            if count > 1000 {
                break;
            }
        }

        before
    }
}

// Constants and helpers
impl Term {
    /// Church encoding of true
    pub fn church_true() -> Term {
        let x = fresh_var();
        let y = fresh_var();
        Term::Abstraction {
            binds: x.clone(),
            term: Box::new(Term::Abstraction {
                binds: y,
                term: Box::new(Term::Variable { name: x }),
                location: Location::Main,
            }),
            location: Location::Main,
        }
    }

    /// Church encoding of false
    pub fn church_false() -> Term {
        let x = fresh_var();
        let y = fresh_var();
        Term::Abstraction {
            binds: x,
            term: Box::new(Term::Abstraction {
                binds: y.clone(),
                term: Box::new(Term::Variable { name: y }),
                location: Location::Main,
            }),
            location: Location::Main,
        }
    }

    pub fn id() -> Term {
        let fresh_var = fresh_var();
        Term::Abstraction {
            binds: fresh_var.clone(),
            term: Box::new(Term::Variable { name: fresh_var }),
            location: Location::Main,
        }
    }

    pub fn y() -> Term {
        let f = fresh_var();
        let x = fresh_var();

        let inner = Term::Abstraction {
            binds: x.clone(),
            term: Box::new(Term::Application {
                function: Box::new(Term::Variable { name: f.clone() }),
                argument: Box::new(Term::Application {
                    function: Box::new(Term::Variable { name: x.clone() }),
                    argument: Box::new(Term::Variable { name: x.clone() }),
                    location: Location::Main,
                }),
                location: Location::Main,
            }),
            location: Location::Main,
        };

        Term::Abstraction {
            binds: f.clone(),
            term: Box::new(Term::Application {
                function: Box::new(inner.clone()),
                argument: Box::new(inner.clone()),
                location: Location::Main,
            }),
            location: Location::Main,
        }
    }

    pub fn as_expression(self) -> Term {
        Term::Application {
            function: Box::new(Term::Choice(Choice::skip())),
            argument: Box::new(self),
            location: Location::Main,
        }
    }
}

#[cfg(test)]
// These tests were generated by ChatGPT.
mod tests {
    use super::*;
    use crate::fmc_core::choice::{Constant, Exception};

    fn v(name: &str) -> Term {
        Term::Variable {
            name: name.to_string(),
        }
    }

    fn lam(param: &str, body: Term) -> Term {
        Term::Abstraction {
            binds: param.to_string(),
            term: Box::new(body),
            location: Location::Main,
        }
    }

    fn app(f: Term, a: Term) -> Term {
        Term::Application {
            function: Box::new(f),
            argument: Box::new(a),
            location: Location::Main,
        }
    }

    fn sorted(mut h: HashSet<String>) -> Vec<String> {
        let mut v: Vec<_> = h.drain().collect();
        v.sort();
        v
    }

    fn sorted_locations(mut h: HashSet<Location>) -> Vec<Location> {
        let mut v: Vec<_> = h.drain().collect();
        v.sort_by_key(|location| location.to_string());
        v
    }

    #[test]
    fn free_vars_variable() {
        let t = v("x");
        let fv = t.free_variables();
        assert_eq!(sorted(fv), vec!["x"]);
    }

    #[test]
    fn free_vars_abs_and_app() {
        // (\x. x y) z → free = {y, z}
        let term = app(lam("x", app(v("x"), v("y"))), v("z"));
        let fv = term.free_variables();
        assert_eq!(sorted(fv), vec!["y", "z"]);
    }

    #[test]
    fn free_vars_nested() {
        // \x.\y. x z → free = {z}
        let term = lam("x", lam("y", app(v("x"), v("z"))));
        let fv = term.free_variables();
        assert_eq!(sorted(fv), vec!["z"]);
    }

    #[test]
    fn substitute_simple_no_capture() {
        // substitute x → z in (x y) = (z y)
        let term = app(v("x"), v("y"));
        let out = term.substitute("x", v("z"));
        assert_eq!(out, app(v("z"), v("y")));
    }

    #[test]
    fn substitute_bound_variable_ignored() {
        // substitute x → z in (\x. x y) → unchanged
        let term = lam("x", app(v("x"), v("y")));
        let out = term.substitute("x", v("z"));
        assert_eq!(out, lam("x", app(v("x"), v("y"))));
    }

    #[test]
    fn substitute_capture_avoiding() {
        // substitute y → x in (\x. y)
        //
        // MUST NOT produce (\x. x) because that captures free x
        let term = lam("x", v("y"));
        let out = term.substitute("y", v("x"));

        match out {
            Term::Abstraction { binds, term, .. } => {
                assert_ne!(binds, "x", "Failed to avoid capture!");
                assert_eq!(*term, v("x"));
            }
            _ => panic!("Expected abstraction"),
        }
    }

    #[test]
    fn substitute_deep_capture_avoiding() {
        // substitute z → (x y) in (\x. \y. z)
        let term = lam("x", lam("y", v("z")));
        let repl = app(v("x"), v("y"));
        let out = term.substitute("z", repl.clone());

        match out {
            Term::Abstraction {
                binds: b1,
                term: body1,
                ..
            } => {
                assert_ne!(b1, "x");
                assert_ne!(b1, "y");

                match *body1 {
                    Term::Abstraction {
                        binds: b2,
                        term: body2,
                        ..
                    } => {
                        assert_ne!(b2, "x");
                        assert_ne!(b2, "y");
                        assert_eq!(*body2, repl);
                    }
                    _ => panic!("Expected nested abstraction"),
                }
            }
            _ => panic!("Expected abstraction"),
        }
    }

    #[test]
    fn beta_reduce_identity() {
        let id = lam("x", v("x"));
        let term = app(id.clone(), v("y"));
        let nf = term.normal_order();
        assert_eq!(nf, v("y"));
    }

    #[test]
    fn beta_reduce_nested() {
        // ((\x. \y. x) a) b → (\y. a) b → a
        let term = app(app(lam("x", lam("y", v("x"))), v("a")), v("b"));
        let nf = term.compute_reduction(false);
        assert_eq!(nf, v("a"));
    }

    #[test]
    fn beta_reduce_avoids_capture() {
        // ((\x. \y. x y) y) z → y z
        let term = app(app(lam("x", lam("y", app(v("x"), v("y")))), v("y")), v("z"));
        let nf = term.compute_reduction(false);
        assert_eq!(nf, app(v("y"), v("z")));
    }

    #[test]
    fn beta_reduce_normal_form_idempotent() {
        let nf = lam("x", v("x"));
        assert_eq!(nf.clone().compute_reduction(false), nf);
    }

    #[test]
    fn expand_operations_recursively_rewrites_nested_operations() {
        let term = Term::Case {
            term: Box::new(Term::Operation(Operation::Sequence {
                first: Box::new(v("x")),
                second: Box::new(Term::Operation(Operation::Constant {
                    choice: Choice::Constant(Constant::Integer(5)),
                })),
            })),
            exit: Choice::Exception(Exception::Break),
            then: Box::new(Term::Loop {
                term: Box::new(Term::Operation(Operation::Throw {
                    error_code: Choice::Exception(Exception::Return),
                })),
                branch: Choice::skip(),
            }),
        };

        let expanded = term.expand_operations();

        assert_eq!(
            expanded,
            Term::Case {
                term: Box::new(Term::Case {
                    term: Box::new(v("x")),
                    exit: Choice::skip(),
                    then: Box::new(Term::Application {
                        function: Box::new(Term::Choice(Choice::skip())),
                        argument: Box::new(Term::Choice(Choice::Constant(Constant::Integer(5)))),
                        location: Location::Main,
                    }),
                }),
                exit: Choice::Exception(Exception::Break),
                then: Box::new(Term::Loop {
                    term: Box::new(Term::Choice(Choice::Exception(Exception::Return))),
                    branch: Choice::skip(),
                }),
            }
        );
    }

    #[test]
    fn get_bound_variables_collects_from_nested_terms() {
        let term = Term::Case {
            term: Box::new(lam("x", v("x"))),
            exit: Choice::skip(),
            then: Box::new(Term::Loop {
                term: Box::new(lam("y", app(v("x"), v("y")))),
                branch: Choice::Exception(Exception::Break),
            }),
        };

        assert_eq!(sorted(term.get_bound_variables()), vec!["x", "y"]);
    }

    #[test]
    fn rename_variable_updates_binders_and_occurrences() {
        let term = Term::Case {
            term: Box::new(lam("x", app(v("x"), v("y")))),
            exit: Choice::skip(),
            then: Box::new(Term::Loop {
                term: Box::new(v("x")),
                branch: Choice::skip(),
            }),
        };

        let renamed = term.rename_variable("x", "z");

        assert_eq!(
            renamed,
            Term::Case {
                term: Box::new(lam("z", app(v("z"), v("y")))),
                exit: Choice::skip(),
                then: Box::new(Term::Loop {
                    term: Box::new(v("z")),
                    branch: Choice::skip(),
                }),
            }
        );
    }

    #[test]
    fn freshen_variables_replaces_all_bound_names() {
        let term = lam("x", lam("y", app(v("x"), v("y"))));
        let freshened = term.freshen_variables();

        match freshened {
            Term::Abstraction {
                binds: outer,
                term: outer_body,
                ..
            } => {
                assert_ne!(outer, "x");
                assert_ne!(outer, "y");

                match *outer_body {
                    Term::Abstraction {
                        binds: inner,
                        term: inner_body,
                        ..
                    } => {
                        assert_ne!(inner, "x");
                        assert_ne!(inner, "y");
                        assert_ne!(inner, outer);
                        assert_eq!(*inner_body, app(v(&outer), v(&inner)));
                    }
                    _ => panic!("expected nested abstraction"),
                }
            }
            _ => panic!("expected abstraction"),
        }
    }

    #[test]
    fn get_all_locations_collects_unique_locations() {
        let term = Term::Case {
            term: Box::new(Term::Abstraction {
                binds: "x".to_string(),
                term: Box::new(Term::Application {
                    function: Box::new(v("f")),
                    argument: Box::new(v("x")),
                    location: Location::Cell("c1".to_string()),
                }),
                location: Location::Local("l1".to_string()),
            }),
            exit: Choice::skip(),
            then: Box::new(Term::Loop {
                term: Box::new(Term::Application {
                    function: Box::new(v("g")),
                    argument: Box::new(v("y")),
                    location: Location::Output,
                }),
                branch: Choice::skip(),
            }),
        };

        assert_eq!(
            sorted_locations(term.get_all_locations()),
            vec![
                Location::Cell("c1".to_string()),
                Location::Local("l1".to_string()),
                Location::Output,
            ]
        );
    }

    #[test]
    fn freshen_locations_renames_only_local_locations() {
        let term = Term::Abstraction {
            binds: "x".to_string(),
            term: Box::new(Term::Application {
                function: Box::new(v("f")),
                argument: Box::new(v("x")),
                location: Location::Local("l1".to_string()),
            }),
            location: Location::Cell("c1".to_string()),
        };

        let freshened = term.freshen_locations();

        match freshened {
            Term::Abstraction { location, term, .. } => {
                assert_eq!(location, Location::Cell("c1".to_string()));
                match *term {
                    Term::Application { location, .. } => match location {
                        Location::Local(name) => assert_ne!(name, "l1"),
                        other => panic!("expected local location, got {other:?}"),
                    },
                    other => panic!("expected application, got {other:?}"),
                }
            }
            other => panic!("expected abstraction, got {other:?}"),
        }
    }

    #[test]
    fn rename_cells_updates_only_matching_cell_locations() {
        let term = Term::Case {
            term: Box::new(Term::Abstraction {
                binds: "x".to_string(),
                term: Box::new(v("x")),
                location: Location::Cell("c1".to_string()),
            }),
            exit: Choice::skip(),
            then: Box::new(Term::Application {
                function: Box::new(v("f")),
                argument: Box::new(v("y")),
                location: Location::Cell("c2".to_string()),
            }),
        };

        let renamed = term.rename_cells("c1".to_string(), "c9".to_string());

        assert_eq!(
            renamed,
            Term::Case {
                term: Box::new(Term::Abstraction {
                    binds: "x".to_string(),
                    term: Box::new(v("x")),
                    location: Location::Cell("c9".to_string()),
                }),
                exit: Choice::skip(),
                then: Box::new(Term::Application {
                    function: Box::new(v("f")),
                    argument: Box::new(v("y")),
                    location: Location::Cell("c2".to_string()),
                }),
            }
        );
    }

    #[test]
    fn switch_cell_to_local_converts_cell_locations_recursively() {
        let term = Term::Loop {
            term: Box::new(Term::Application {
                function: Box::new(Term::Abstraction {
                    binds: "x".to_string(),
                    term: Box::new(v("x")),
                    location: Location::Cell("c1".to_string()),
                }),
                argument: Box::new(v("y")),
                location: Location::Cell("c2".to_string()),
            }),
            branch: Choice::skip(),
        };

        let switched = term.switch_cell_to_local();

        assert_eq!(
            switched,
            Term::Loop {
                term: Box::new(Term::Application {
                    function: Box::new(Term::Abstraction {
                        binds: "x".to_string(),
                        term: Box::new(v("x")),
                        location: Location::Local("c1".to_string()),
                    }),
                    argument: Box::new(v("y")),
                    location: Location::Local("c2".to_string()),
                }),
                branch: Choice::skip(),
            }
        );
    }

    #[test]
    fn rename_locals_updates_only_matching_local_locations() {
        let term = Term::Case {
            term: Box::new(Term::Abstraction {
                binds: "x".to_string(),
                term: Box::new(v("x")),
                location: Location::Local("l1".to_string()),
            }),
            exit: Choice::skip(),
            then: Box::new(Term::Application {
                function: Box::new(v("f")),
                argument: Box::new(v("y")),
                location: Location::Local("l2".to_string()),
            }),
        };

        let renamed = term.rename_locals("l1".to_string(), "l9".to_string());

        assert_eq!(
            renamed,
            Term::Case {
                term: Box::new(Term::Abstraction {
                    binds: "x".to_string(),
                    term: Box::new(v("x")),
                    location: Location::Local("l9".to_string()),
                }),
                exit: Choice::skip(),
                then: Box::new(Term::Application {
                    function: Box::new(v("f")),
                    argument: Box::new(v("y")),
                    location: Location::Local("l2".to_string()),
                }),
            }
        );
    }

    #[test]
    fn church_true_selects_first_argument_after_reduction() {
        let term = app(app(Term::church_true(), v("a")), v("b"));
        assert_eq!(term.compute_reduction(false), v("a"));
    }

    #[test]
    fn church_false_selects_second_argument_after_reduction() {
        let term = app(app(Term::church_false(), v("a")), v("b"));
        assert_eq!(term.compute_reduction(false), v("b"));
    }

    #[test]
    fn id_is_an_identity_abstraction() {
        match Term::id() {
            Term::Abstraction {
                binds,
                term,
                location,
            } => {
                assert_eq!(location, Location::Main);
                assert_eq!(*term, v(&binds));
            }
            other => panic!("expected abstraction, got {other:?}"),
        }
    }

    #[test]
    fn y_combinator_has_expected_self_application_shape() {
        match Term::y() {
            Term::Abstraction {
                binds: f,
                term,
                location,
            } => {
                assert_eq!(location, Location::Main);
                match *term {
                    Term::Application {
                        function,
                        argument,
                        location,
                    } => {
                        assert_eq!(location, Location::Main);
                        assert_eq!(*function, *argument);

                        match *function {
                            Term::Abstraction {
                                binds: x,
                                term,
                                location,
                            } => {
                                assert_eq!(location, Location::Main);
                                assert_eq!(
                                    *term,
                                    Term::Application {
                                        function: Box::new(v(&f)),
                                        argument: Box::new(Term::Application {
                                            function: Box::new(v(&x)),
                                            argument: Box::new(v(&x)),
                                            location: Location::Main,
                                        }),
                                        location: Location::Main,
                                    }
                                );
                            }
                            other => panic!("expected inner abstraction, got {other:?}"),
                        }
                    }
                    other => panic!("expected self-application, got {other:?}"),
                }
            }
            other => panic!("expected abstraction, got {other:?}"),
        }
    }
}
