use std::env;
use std::fs;
use tiny_wat_interpreter::Module;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <file.wat> <function_name> [arg1 arg2 ...]",
            args[0]
        );
        std::process::exit(1);
    }

    let file_path = &args[1];
    let func_name = &args[2];

    let mut fn_args = Vec::new();
    for arg in &args[3..] {
        match arg.parse::<i32>() {
            Ok(v) => fn_args.push(v),
            Err(e) => {
                eprintln!("Failed to parse argument '{}' as i32: {}", arg, e);
                std::process::exit(1);
            }
        }
    }

    let wat_content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read WAT file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let module = match Module::parse_wat(&wat_content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to parse WAT module: {}", e);
            std::process::exit(1);
        }
    };

    match module.invoke(func_name, &fn_args) {
        Ok(res) => println!("{}", res),
        Err(e) => {
            eprintln!("Execution error: {}", e);
            std::process::exit(1);
        }
    }
}
