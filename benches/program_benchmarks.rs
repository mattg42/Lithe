use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use Lithe::{
    fmc_core::Term,
    interpreter::Interpreter,
    machines::{
        KrivineMachine, Machine, StackMachine,
        machine::{MachineType, StepResult},
        runtime_io::parse_input_term,
    },
};

const FIB_10_PROGRAM: &str = r#"
fn fib(n) {
    if ($n <= 1) {
        return 1;
    } else {
        return fib($n - 1) + fib($n - 2);
    }
}

print fib(10);
"#;

const PRIMES_PROGRAM: &str = include_str!("../primes.lithe");

struct BenchmarkCase {
    name: &'static str,
    program: &'static str,
    inputs: &'static [&'static str],
}

fn bench_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            name: "fib_n10",
            program: FIB_10_PROGRAM,
            inputs: &[],
        },
        BenchmarkCase {
            name: "primes_n10",
            program: PRIMES_PROGRAM,
            inputs: &["10"],
        },
        BenchmarkCase {
            name: "primes_n100",
            program: PRIMES_PROGRAM,
            inputs: &["100"],
        },
    ]
}

fn compile_program(program: &str, optimise: bool) -> Term {
    Interpreter::new(true, MachineType::Stack)
        .compile(program.to_string(), optimise, false)
        .unwrap()
}

fn parse_inputs(inputs: &[&str]) -> Vec<Term> {
    inputs
        .iter()
        .map(|input| parse_input_term(input).unwrap())
        .collect()
}

fn run_once(machine_type: &MachineType, term: Term, inputs: &[Term]) -> StepResult {
    match machine_type {
        MachineType::Stack => {
            let mut machine = StackMachine::new(term);
            machine.set_silent(true);
            if !inputs.is_empty() {
                machine.seed_input(inputs.to_vec());
            }
            machine.run(false)
        }
        MachineType::Krivine => {
            let mut machine = KrivineMachine::new(term);
            machine.set_silent(true);
            if !inputs.is_empty() {
                machine.seed_input(inputs.to_vec());
            }
            machine.run(false)
        }
    }
}

fn benchmark_programs(c: &mut Criterion) {
    let cases = bench_cases();

    for machine_type in [MachineType::Stack, MachineType::Krivine] {
        for optimise in [false, true] {
            let machine_name = match machine_type {
                MachineType::Stack => "stack",
                MachineType::Krivine => "krivine",
            };
            let optimisation_name = if optimise { "optimised" } else { "unoptimised" };

            let mut group = c.benchmark_group(format!("{machine_name}/{optimisation_name}"));

            for case in &cases {
                let compiled = compile_program(case.program, optimise);
                let inputs = parse_inputs(case.inputs);

                group.bench_with_input(BenchmarkId::from_parameter(case.name), case, |b, case| {
                    b.iter(|| {
                        let result = run_once(&machine_type, compiled.clone(), &inputs);
                        assert_eq!(
                            result,
                            StepResult::Stop,
                            "benchmark case `{}` did not terminate",
                            case.name
                        );
                    });
                });
            }

            group.finish();
        }
    }
}

criterion_group!(benches, benchmark_programs);
criterion_main!(benches);
