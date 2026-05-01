# Lithe

Lithe is a small, lightweight surface language that compiles to FMC terms and runs on abstract machines.

This repository contains a Rust implementation of the Lithe compiler and interpreter. Programs can run on one of two machines:

- a stack machine
- a Krivine machine

The project includes:

- a lexer and parser for Lithe
- an FMC core term representation
- two execution machines
- unit and integration tests
- Criterion benchmarks

## Build

```bash
cargo build
```

## Run

Run a Lithe program with the stack machine:

```bash
cargo run -- fib.lithe
```

Run a Lithe program with the Krivine machine:

```bash
cargo run -- --krivine fib.lithe
```

Enable compile-time reduction before execution:

```bash
cargo run -- --optimise fib.lithe
```

Enable the `rnd` standard library:

```bash
cargo run -- --rnd guessing.lithe
```

Print the machine trace:

```bash
cargo run -- --trace recurse.lithe
```

## CLI

```text
fmc_interpreter [OPTIONS] <PATH>
```

Options:

- `--krivine`: use the Krivine machine instead of the stack machine
- `--trace`: print the machine trace
- `--optimise`: reduce the compiled term before execution
- `--rnd`: prepend the `rnd` standard library

## Example Programs

`fib.lithe`

```fmc
fn fib(n) {
    if ($n <= 1) {
        return 1;
    } else {
        return fib($n - 1) + fib($n - 2);
    }
}

print fib(5);
```

`primes.lithe`

```fmc
n := input;
i := 2;

while ($i <= $n) {
    isprime := true;

    j := 2;
    while ($j <= $i/2) {
        if ($i % $j == 0) {
            isprime := false;
            break;
        }
        j := $j + 1;
    }

    if ($isprime) {
        print $i;
    }

    i := $i + 1;
}
```

`guessing.lithe`

```fmc
a := rnd_int(1, 10);

guess := input;
while ($guess != $a) {
    print $guess == $a;
    guess := input;
}
```

Run the guessing program with `--rnd`, since `rnd_bool()` and `rnd_int(...)` are provided by the optional Lithe standard library.

## Lithe Language

Supported high-level features include:

- integer and boolean values
- variables via `$name`
- assignment with `:=`
- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparisons: `<`, `<=`, `>`, `>=`, `==`, `!=`
- logic: `!`, `&&`, `||`
- `if` / `else`
- `while`
- `break`
- `return`
- top-level function declarations
- recursion
- `print`
- `input`
- embedded raw FMC terms using `\ ... \`

The parser grammar lives in [src/interpreter/parser.rs](src/interpreter/parser.rs).

## Randomness

Randomness is opt-in at compile time through `--rnd`.

When enabled, the interpreter prepends a small Lithe standard library from [src/interpreter/rnd.lithe](src/interpreter/rnd.lithe), which defines:

- `rnd_bool()`
- `rnd_int(low, high)`

`rnd_int(low, high)` is inclusive.

## Tests

Run the full test suite:

```bash
cargo test
```

Important test files:

- [tests/program_operators.rs](tests/program_operators.rs): end-to-end program tests on both machines
- [tests/ast_roundtrip.rs](tests/ast_roundtrip.rs): pretty-print / raw-FMC round-trip tests

## Benchmarks

Compile the benchmark target:

```bash
cargo bench --bench program_benchmarks --no-run
```

Run benchmarks:

```bash
cargo bench --bench program_benchmarks
```

The benchmark matrix currently covers:

- `fib(10)`
- `primes` with input `10`
- `primes` with input `100`

for each of:

- stack vs Krivine
- optimised vs unoptimised

See [benches/program_benchmarks.rs](benches/program_benchmarks.rs).

## Current Limitations

- top-level mutual recursion is not currently supported
- exact raw FMC round-tripping cannot always preserve `Local` vs `Cell` locations because the printed syntax is ambiguous there
- Compile and runtime errors are quite weak

## Project Layout

- [src/interpreter](src/interpreter): lexer, parser, and compile pipeline
- [src/fmc_core](src/fmc_core): core term, operations, choices, specials, locations
- [src/machines](src/machines): stack and Krivine machines plus runtime helpers
- [tests](tests): integration tests
- [benches](benches): Criterion benchmarks
