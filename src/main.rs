use std::env;

mod synacor;
use self::synacor::Vm;

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut prog = Vm::new(args.get(1).map_or("challenge.bin", |s| &s));

    prog.run();
}
