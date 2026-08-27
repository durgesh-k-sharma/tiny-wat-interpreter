use tiny_wat_interpreter::Module;

#[test]
fn test_fib() {
    let wat = include_str!("fib.wat");
    let module = Module::parse_wat(wat).expect("Failed to parse fib.wat");

    assert_eq!(module.invoke("fib", &[0]).unwrap(), 0);
    assert_eq!(module.invoke("fib", &[1]).unwrap(), 1);
    assert_eq!(module.invoke("fib", &[2]).unwrap(), 1);
    assert_eq!(module.invoke("fib", &[10]).unwrap(), 55);
    assert_eq!(module.invoke("fib", &[15]).unwrap(), 610);
    assert_eq!(module.invoke("fib", &[20]).unwrap(), 6765);
}

#[test]
fn test_tak() {
    let wat = include_str!("tak.wat");
    let module = Module::parse_wat(wat).expect("Failed to parse tak.wat");

    assert_eq!(module.invoke("tak", &[6, 4, 2]).unwrap(), 3);
    assert_eq!(module.invoke("tak", &[18, 12, 6]).unwrap(), 7);
}

#[test]
fn test_ackermann() {
    let wat = include_str!("ackermann.wat");
    let module = Module::parse_wat(wat).expect("Failed to parse ackermann.wat");

    assert_eq!(module.invoke("ackermann", &[0, 0]).unwrap(), 1);
    assert_eq!(module.invoke("ackermann", &[1, 1]).unwrap(), 3);
    assert_eq!(module.invoke("ackermann", &[2, 2]).unwrap(), 7);
    assert_eq!(module.invoke("ackermann", &[3, 3]).unwrap(), 61);
}

#[test]
fn test_gcd() {
    let wat = include_str!("gcd.wat");
    let module = Module::parse_wat(wat).expect("Failed to parse gcd.wat");

    assert_eq!(module.invoke("gcd", &[48, 18]).unwrap(), 6);
    assert_eq!(module.invoke("gcd", &[101, 10]).unwrap(), 1);
    assert_eq!(module.invoke("gcd", &[54, 24]).unwrap(), 6);
}

#[test]
fn test_is_prime() {
    let wat = include_str!("is_prime.wat");
    let module = Module::parse_wat(wat).expect("Failed to parse is_prime.wat");

    assert_eq!(module.invoke("is_prime", &[1]).unwrap(), 0);
    assert_eq!(module.invoke("is_prime", &[2]).unwrap(), 1);
    assert_eq!(module.invoke("is_prime", &[3]).unwrap(), 1);
    assert_eq!(module.invoke("is_prime", &[4]).unwrap(), 0);
    assert_eq!(module.invoke("is_prime", &[29]).unwrap(), 1);
    assert_eq!(module.invoke("is_prime", &[100]).unwrap(), 0);
    assert_eq!(module.invoke("is_prime", &[541]).unwrap(), 1);
}

#[test]
fn test_runtime_errors_and_edge_cases() {
    let wat = r#"
    (module
      (func $div (export "div") (param $a i32) (param $b i32) (result i32)
        (i32.rem_s (local.get $a) (local.get $b))
      )
      (func $nested (export "nested") (param $x i32) (result i32)
        (if (result i32)
          (i32.lt_s (local.get $x) (i32.const 0))
          (then
            (i32.sub (i32.const 0) (local.get $x))
          )
          (else
            (local.get $x)
          )
        )
      )
      (func $const_hex (export "const_hex") (result i32)
        (i32.add (i32.const 0x10) (i32.const -5))
      )
    )
    "#;
    let module = Module::parse_wat(wat).expect("Failed to parse edge cases wat");

    // Division by zero returns Err
    assert!(module.invoke("div", &[10, 0]).is_err());

    // Absolute value logic via if-else
    assert_eq!(module.invoke("nested", &[-42]).unwrap(), 42);
    assert_eq!(module.invoke("nested", &[42]).unwrap(), 42);

    // Hex and negative constant arithmetic (16 + (-5) = 11)
    assert_eq!(module.invoke("const_hex", &[]).unwrap(), 11);

    // Missing export
    assert!(module.invoke("non_existent", &[]).is_err());

    // Arity mismatch
    assert!(module.invoke("nested", &[]).is_err());
    assert!(module.invoke("nested", &[1, 2]).is_err());
}
