Emulates my [Ember CPU architecture](./ember/architecture.md).
Note that this project is in a very early state and mostly just doesn't work.

Programs are written in `.instr` files. See the (outdated) language specification [here](./ember/language_specification.instr).
See examples [here](./ember/examples).

# Running

Run using `cargo run [args]`.

Examples:
- `cargo run comp ./ember/examples/fibonacci.instr`
- `cargo run run ./ember/examples/fibonacci.cpu`
- `cargo run run ./ember/examples/print_str.instr`

Running a program opens a CLI. Type `help` for a list of commands.

## Command Line Interface

`<exe> comp <path> [outpath]`
*compiles a .instr file into a runnable .cpu file*

`<exe> run <path>`
*runs a .instr or .cpu file*
Run `help` for a list of commands

`<exe> norm <path>`
*normalizes a .instr file, compiling its jumps, macros and inlines*
