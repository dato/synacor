use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/*
 * Public struct and impl.
 */
pub struct Vm {
    pc: usize,
    bin: Vec<u16>,
    regs: Vec<u16>,
    stack: Vec<u16>,
    input: Vec<u8>,
}

impl Vm {
    pub fn new(filename: &str) -> Vm {
        let f = File::open(filename).unwrap();
        let info = f.metadata().unwrap();
        let mut buf = BufReader::new(f);
        let mut bin = vec![0; info.len() as usize / 2]; // Assume LEN%2 == 0.

        // Read buf as a stream of 16-bit LE integers into bin.
        buf.read_u16_into::<LittleEndian>(&mut bin).unwrap();

        Vm {
            bin,
            pc: 0,
            regs: vec![0; 8],
            stack: Vec::new(),
            input: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        let mut state = State::Running;
        while let State::Running = state {
            let op = self.next_op();
            state = self.run_op(op);
        }
    }
}

/*
 * Private implementation.
 */

enum State {
    Running,
    Halted,
}

#[derive(Debug)]
enum Op {
    Hlt,                // 0: Stop execution and terminate the program
    Set(u16, u16),      // 1: Set register <a> to the value of <b>
    Push(u16),          // 2: Push <a> onto the stack
    Pop(u16),           // 3: Remove the top element from the stack and write it into <a>
    Eq(u16, u16, u16),  // 4: Set <a> to 1 if <b> is equal to <c>; set it to 0 otherwise
    Gt(u16, u16, u16),  // 5: Set <a> to 1 if <b> is greater than <c>; set it to 0 otherwise
    Jmp(u16),           // 6: Jump to <a>
    Jt(u16, u16),       // 7: If <a> is nonzero, jump to <b>
    Jf(u16, u16),       // 8: If <a> is zero, jump to <b>
    Add(u16, u16, u16), // 9: Assign into <a> the sum of <b> and <c> (modulo 32768)
    Mul(u16, u16, u16), // 10: Store into <a> the product of <b> and <c> (modulo 32768)
    Mod(u16, u16, u16), // 11: Store into <a> the remainder of <b> divided by <c>
    And(u16, u16, u16), // 12: Stores into <a> the bitwise and of <b> and <c>
    Or(u16, u16, u16),  // 13: Stores into <a> the bitwise or of <b> and <c>
    Not(u16, u16),      // 14: Stores 15-bit bitwise inverse of <b> in <a>
    Rmem(u16, u16),     // 15: Read memory at address <b> and write it to <a>
    Wmem(u16, u16),     // 16: Write the value from <b> into memory at address <a>
    Call(u16),          // 17: Write the address of next instruction to the stack and jump to <a>
    Ret,                // 18: Remove top element from the stack and jump to it; empty stack = halt
    Out(u16),           // 19: Write the character represented by ascii code <a> to the terminal
    In(u16),            // 20: Read a character from the terminal and write its ascii code to <a>
    Noop,               // 21: No operation
}

// TODO: Abstract better than this.
static ARITY: [usize; 22] = [
    0, 2, 1, 1, 3, 3, 1, 2, 2, 3, 3, 3, 3, 3, 2, 2, 2, 1, 0, 1, 1, 0,
];

impl Vm {
    fn next_op(&mut self) -> Op {
        let i = self.pc;
        let b = &self.bin;
        self.pc += 1 + ARITY[b[i] as usize];

        match b[i] {
            0 => Op::Hlt,
            1 => Op::Set(b[i + 1], b[i + 2]),
            2 => Op::Push(b[i + 1]),
            3 => Op::Pop(b[i + 1]),
            4 => Op::Eq(b[i + 1], b[i + 2], b[i + 3]),
            5 => Op::Gt(b[i + 1], b[i + 2], b[i + 3]),
            6 => Op::Jmp(b[i + 1]),
            7 => Op::Jt(b[i + 1], b[i + 2]),
            8 => Op::Jf(b[i + 1], b[i + 2]),
            9 => Op::Add(b[i + 1], b[i + 2], b[i + 3]),
            10 => Op::Mul(b[i + 1], b[i + 2], b[i + 3]),
            11 => Op::Mod(b[i + 1], b[i + 2], b[i + 3]),
            12 => Op::And(b[i + 1], b[i + 2], b[i + 3]),
            13 => Op::Or(b[i + 1], b[i + 2], b[i + 3]),
            14 => Op::Not(b[i + 1], b[i + 2]),
            15 => Op::Rmem(b[i + 1], b[i + 2]),
            16 => Op::Wmem(b[i + 1], b[i + 2]),
            17 => Op::Call(b[i + 1]),
            18 => Op::Ret,
            19 => Op::Out(b[i + 1]),
            20 => Op::In(b[i + 1]),
            21 => Op::Noop,
            code => {
                panic!("unknown opcode {:?}", code);
            }
        }
    }

    fn set(&mut self, reg: u16, val: u16) {
        self.regs[reg as usize % 32768] = val % 32768;
    }

    fn run_op(&mut self, op: Op) -> State {
        let v = |x: u16| match x {
            0...32767 => x,
            32768...32775 => self.regs[x as usize % 32768],
            _ => panic!("invalid number {}", x),
        };
        let int = |b: bool| if b { 1 } else { 0 };

        match op {
            Op::Hlt => {
                return State::Halted;
            }
            Op::Set(a, b) => {
                self.set(a, v(b));
            }
            Op::Push(a) => {
                self.stack.push(v(a));
            }
            Op::Pop(a) => {
                let pop = self.stack.pop();
                self.set(a, pop.unwrap());
            }
            Op::Eq(a, b, c) => {
                self.set(a, int(v(b) == v(c)));
            }
            Op::Gt(a, b, c) => {
                self.set(a, int(v(b) > v(c)));
            }
            Op::Jmp(a) => {
                self.pc = v(a) as usize;
            }
            Op::Jt(a, b) => {
                if v(a) != 0 {
                    self.pc = v(b) as usize
                }
            }
            Op::Jf(a, b) => {
                if v(a) == 0 {
                    self.pc = v(b) as usize
                }
            }
            Op::Add(a, b, c) => {
                self.set(a, v(b) + v(c));
            }
            Op::Mul(a, b, c) => {
                self.set(a, v(b) * v(c));
            }
            Op::Mod(a, b, c) => {
                self.set(a, v(b) % v(c));
            }
            Op::And(a, b, c) => {
                self.set(a, v(b) & v(c));
            }
            Op::Or(a, b, c) => {
                self.set(a, v(b) | v(c));
            }
            Op::Not(a, b) => {
                self.set(a, !v(b));
            }
            Op::Rmem(a, b) => {
                self.set(a, self.bin[v(b) as usize]);
            }
            Op::Wmem(a, b) => {
                let a = v(a);
                let b = v(b);
                self.bin[a as usize] = b;
            }
            Op::Call(a) => {
                let a = v(a) as usize;
                self.stack.push(self.pc as u16);
                self.pc = a;
            }
            Op::Ret => match self.stack.pop() {
                None => return State::Halted,
                Some(addr) => self.pc = addr as usize,
            },
            Op::Out(a) => {
                print!("{}", a as u8 as char);
            }
            Op::In(a) => {
                if self.input.is_empty() {
                    let stdin = io::stdin();
                    let mut handle = stdin.lock();
                    handle.read_until('\n' as u8, &mut self.input).unwrap();
                    self.input.reverse();
                }
                match self.input.pop() {
                    Some(c) => self.set(a, c as u16),
                    None => return State::Halted,
                }
            }
            Op::Noop => {}
        };
        State::Running
    }
}
