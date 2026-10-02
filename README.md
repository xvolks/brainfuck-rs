# Brainfuck Interpreter

A simple Brainfuck interpreter and compiler written in Rust.

## Usage

```bash
# Run with a Brainfuck program file as argument
cargo run -- [--no-jit] "<program.bf>"

# Or Hello, World mode
cargo run --release
```

## Commands

- `cargo run -- "<bf program file>"` - executes the Brainfuck program in this file
- Program files can be passed as a single argument or multiple arguments
- The option `--no-jit` disables the native code compilation and run in interpreted mode

## Features

- Basic Brainfuck operations: `> < + - . , [ ]`
- Loop handling with jump optimizations
- Interactive character input
- Hello World and quine demonstrations

## Weaknesses & Future Work

The following limitations exist and are planned for future improvements:

- **No bracket validation**: Unbalanced `[` and `]` cause panics during parsing; robust error handling is needed
- **Fixed memory size**: Memory is capped at 1024 cells (`MAX_MEM`), programs requiring more cells will fail
- **No memory bounds checking**: The data pointer can move negative or beyond `MAX_MEM`, causing out-of-bounds access or wrapping behavior that may be unexpected
- **Platform-limited input**: Uses the `getch` crate for single-character input, which may not work on all platforms
- **No optimization**: Loops and cell operations are not optimized for performance
- **Limited test coverage**: Existing tests cover basic cases but do not exhaustively verify edge conditions
- **No --help or usage information**: Entering without arguments prints a hardcoded program rather than showing usage
- **Character output may be non-printable**: The `OUT` operation casts cell values directly to `char`, which may produce unexpected results for non-printable ASCII
- **No REPL or interactive mode beyond single-key input**: Would benefit from a proper REPL with multi-line support
- **Partial cross-platform tests**: Only tested on Arm64 macOS and x86_64/aarch64 Linux.
