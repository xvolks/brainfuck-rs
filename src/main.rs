mod jit;

use std::{env::args, fs, path::PathBuf};

const COMPRESS: bool = cfg!(feature = "compress");

#[derive(Clone, Debug)]
struct Jumps {
    start: usize,
    end: usize,
}
impl Jumps {
    fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum Op {
    Inc(u32),
    Dec(u32),
    Left(u32),
    Right(u32),
    Out(u32),
    In(u32),
    Jz(usize),
    Jnz(usize),
}

impl Op {
    pub fn parse(source: &str) -> Vec<Self> {
        let mut ops: Vec<Op> = vec![];
        // Invalid op as the first one
        let mut last_op = None;
        let mut operand = 0;
        let mut stack = Stack::new();
        for c in source.chars() {
            let op = match c {
                '+' => Op::Inc(1),
                '-' => Op::Dec(1),
                '<' => Op::Left(1),
                '>' => Op::Right(1),
                '.' => Op::Out(1),
                ',' => Op::In(1),
                '[' => Op::Jz(0),
                ']' => Op::Jnz(0),
                _ => continue,
            };
            if COMPRESS {
                let create = if op.is_jump() {
                    true
                } else if last_op != Some(op) {
                    if last_op.is_none() {
                        // do not add the invalid op
                        false
                    } else {
                        true
                    }
                } else {
                    false
                };
                let next_op = if create {
                    if let Some(last_op) = last_op {
                        ops.push(last_op.create_instruction(operand, &mut stack, ops.len()));
                    }
                    operand = 0;
                    if op.is_jump() {
                        ops.push(op.create_instruction(0, &mut stack, ops.len()));
                        None
                    } else {
                        Some(op)
                    }
                } else {
                    Some(op)
                };
                last_op = next_op;
                if last_op.is_some() {
                    operand += 1;
                }
            } else {
                ops.push(op.create_instruction(1, &mut stack, ops.len()));
            }
        }

        if COMPRESS {
            if let Some(last_op) = last_op {
                ops.push(last_op.create_instruction(operand, &mut stack, ops.len()));
            }
        }
        // Ok all instructions are in place, only the Jz are all pointing to the first instruction.
        // We just have to backpatch the program to Jz to the right spot (after the corresponding Jnz instruction).
        while stack.len() > 0 {
            let jump = stack.pop();
            assert!(jump.end > 0, "Unmanaged Jz {jump:?}!");
            // We retrieve the dummy Jz in the list and replace it with the Jz containing the index of the corresponding Jnz
            ops[jump.start] = Op::Jz(jump.end);
        }

        ops
    }

    #[allow(dead_code)]
    pub fn dump(ops: &[Op]) {
        let mut level = 0;
        for (i, op) in ops.iter().enumerate() {
            println!("{i:04}{}- {:?}", " ".repeat(level), op);
            match op {
                Op::Jz(_) => level += 1,
                Op::Jnz(_) => level -= 1,
                _ => {}
            }
        }
    }

    fn is_jump(&self) -> bool {
        match self {
            Op::Jz(_) | Op::Jnz(_) => true,
            _ => false,
        }
    }

    fn create_instruction(&self, operand: u32, stack: &mut Stack<Jumps>, index: usize) -> Op {
        match self.clone() {
            Op::Inc(_) => Op::Inc(operand),
            Op::Dec(_) => Op::Dec(operand),
            Op::Left(_) => Op::Left(operand),
            Op::Right(_) => Op::Right(operand),
            Op::In(_) => Op::In(operand),
            Op::Out(_) => Op::Out(operand),
            Op::Jz(_) => {
                // Memorise the index of this Jz
                // For now we set 0 here since we still do not know where the corresponding Jnz is.
                stack.push(Jumps::new(index, 0));
                Op::Jz(0)
            }
            Op::Jnz(_) => {
                // Get the index of the Jz instruction
                let mut jump = stack.pop();
                let jz_index = jump.start;
                assert!(jump.end == 0, "Found a Jnz for an already managed Jz!");
                // Save the index of Jnz
                jump.end = index;
                // Push back the index of the Jnz instruction for the backpatching phase
                stack.insert_first(jump);
                Op::Jnz(jz_index)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Stack<T> {
    elements: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    pub fn push(&mut self, el: T) {
        self.elements.push(el);
    }
    pub fn pop(&mut self) -> T {
        self.elements.pop().expect("Unbalanced stack")
    }
    #[allow(dead_code)]
    pub fn peek(&self) -> &T {
        self.elements
            .iter()
            .last()
            .expect("Hu-ho! We should have an element here")
    }
    pub fn insert_first(&mut self, el: T) {
        self.elements.insert(0, el);
    }
}

type Mem = Vec<u8>;
const MAX_MEM: usize = 1024;

struct Cpu {
    memory: Mem,
    ops: Vec<Op>,
    ip: usize,
    head: usize,
    jit: bool,
}

impl Cpu {
    pub fn new(ops: Vec<Op>, jit: bool) -> Self {
        Self {
            memory: vec![0; MAX_MEM],
            ops,
            ip: 0,
            head: 0,
            jit,
        }
    }

    pub fn execute(&mut self) {
        if self.jit {
            return self
                .execute_jit()
                .map_err(|err| println!("Error: {err}"))
                .expect("Failed");
        }
        let getch = getch::Getch::new();
        loop {
            #[cfg(debug_assertions)]
            println!(
                "Executing at ip: {} -> {:?}, value: {} @ addr: {}",
                self.ip, self.ops[self.ip], self.memory[self.head], self.head
            );
            // #[cfg(debug_assertions)]
            // thread::sleep(Duration::from_millis(10));

            match self.ops[self.ip] {
                Op::Dec(n) => self.memory[self.head] = self.memory[self.head].wrapping_sub(n as u8),
                Op::Inc(n) => self.memory[self.head] = self.memory[self.head].wrapping_add(n as u8),
                // This might break if the program wants to access the memory before addr 0
                Op::Left(n) => self.head = self.head.saturating_sub(n as usize),
                // This might break if the program wants to access the memory after addr MAX_MEM
                Op::Right(n) => self.head += n as usize,
                Op::Out(n) => {
                    for _ in 0..n {
                        print!("{}", self.memory[self.head] as char);
                    }
                }
                Op::In(n) => {
                    for _ in 0..n {
                        let s = getch.getch().unwrap();
                        self.memory[self.head] = s;
                    }
                }
                Op::Jz(addr) => {
                    if self.memory[self.head] == 0 {
                        // println!("Jump to {addr}, Memory: {:?}", self.memory);
                        self.ip = addr + 1;
                        if self.ip >= self.ops.len() {
                            // We've jump outside the program
                            break;
                        } else {
                            continue; // Skip the ip++
                        }
                    } else {
                        // println!("Nop, Memory: {:?}", self.memory)
                    }
                }
                Op::Jnz(addr) => {
                    if self.memory[self.head] != 0 {
                        // println!("Jump to {addr}, Memory: {:?}", self.memory);
                        self.ip = addr + 1;
                        continue;
                    } else {
                        // println!("Nop, Memory: {:?}", self.memory)
                    }
                }
            };
            self.ip += 1;
            if self.ip >= self.ops.len() {
                break;
            }
        }
    }
}

mod tests {
    #[test]
    fn test_10() {
        crate::exec_source(format!("{}.[-]{}.", "+".repeat(0x31), "+".repeat(0x30)).as_str());
    }

    #[test]
    pub fn hello_complex() {
        use std::fs;
        let prg = fs::read_to_string("samples/hello.bf").unwrap();
        crate::exec_source(prg.as_str());
        crate::exec_source(prg.as_str());
    }

    #[test]
    pub fn hello_1() {
        use std::fs;
        let prg = fs::read_to_string("samples/hello1.bf").unwrap();
        crate::exec_source(prg.as_str());
        crate::exec_source(prg.as_str());
    }

    #[test]
    pub fn jumps() {
        use std::fs;
        let prg = fs::read_to_string("samples/jumps.bf").unwrap();
        crate::exec_source(prg.as_str());
        crate::exec_source(prg.as_str());
    }

    #[test]
    pub fn hello() {
        crate::exec_source(
            "
        +++++ +++++             initialize counter (cell #0) to 10
        [                       use loop to set the next four cells to 70/100/30/10
            > +++++ ++              add  7 to cell #1
            > +++++ +++++           add 10 to cell #2
            > +++                   add  3 to cell #3
            > +                     add  1 to cell #4
            <<<< -                  decrement counter (cell #0)
        ]
        > ++ .                  print 'H'
        > + .                   print 'e'
        +++++ ++ .              print 'l'
        .                       print 'l'
        +++ .                   print 'o'
        > ++ .                  print ' '
        << +++++ +++++ +++++ .  print 'W'
        > .                     print 'o'
        +++ .                   print 'r'
        ----- - .               print 'l'
        ----- --- .             print 'd'
        > + .                   print '!'
        > .                     print '\n'",
        );
    }

    #[test]
    fn five() {
        crate::exec_source(
            "
        +++++           +++++
            +               +
            +     +         +     +++++
        +++++    +++    +++++
        +         +     +         +++++
        +               +
        +++++           +++++.
        ",
        );
    }

    #[test]
    fn quine() {
        crate::exec_source(
            "
        -->+++>+>+>+>+++++>++>++>->+++>++>+>>>>>>>>>>>>>>>>->++++>>>>->+++>+++>+++>+++>+
        ++>+++>+>+>>>->->>++++>+>>>>->>++++>+>+>>->->++>++>++>++++>+>++>->++>++++>+>+>++
        >++>->->++>++>++++>+>+>>>>>->>->>++++>++>++>++++>>>>>->>>>>+++>->++++>->->->+++>
        >>+>+>+++>+>++++>>+++>->>>>>->>>++++>++>++>+>+++>->++++>>->->+++>+>+++>+>++++>>>
        +++>->++++>>->->++>++++>++>++++>>++[-[->>+[>]++[<]<]>>+[>]<--[++>++++>]+[<]<<++]
        >>>[>]++++>++++[--[+>+>++++<<[-->>--<<[->-<[--->>+<<[+>+++<[+>>++<<]]]]]]>+++[>+
        ++++++++++++++<-]>--.<<<]
        ",
        );
    }

    #[test]
    fn cat() {
        crate::exec_source(",[.,]");
    }
}

fn exec_source(source: &str) {
    exec_source_with_jit(source, true);
}

fn exec_source_with_jit(source: &str, jit: bool) {
    let ops = Op::parse(source);
    if ops.is_empty() {
        eprintln!("no source code provided");
        return;
    }
    #[cfg(debug_assertions)]
    Op::dump(&ops);
    Cpu::new(ops, jit).execute();
    println!();
    println!("Brainfuck program is over.")
}

fn main() {
    if args().len() == 1 {
        exec_source(
            ">++++++++[<+++++++++>-]<.>++++[<+++++++>-]<+.+++++++..+++.>>++++++[<+++++++>-]<++.------------.>++++++[<+++++++++>-]<+.<.+++.------.--------.>>>++++[<++++++++>-]<+.",
        );
    } else {
        let mut jit = true;
        for arg in args().skip(1) {
            if arg == "--no-jit" {
                jit = false;
                continue;
            }

            let source = fs::read_to_string(PathBuf::from(&arg)).expect("Cannot read file {arg}");
            exec_source_with_jit(source.as_str(), jit);
            println!("Exec done!")
        }
    }
}
