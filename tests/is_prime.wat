;; Trial-division primality test: https://en.wikipedia.org/wiki/Primality_test#Pseudocode
(module
  (func $is_prime_helper (param $n i32) (param $d i32) (result i32)
    (if (result i32)
      (i32.lt_s (local.get $n) (i32.mul (local.get $d) (local.get $d)))
      (then
        (i32.const 1)
      )
      (else
        (if (result i32)
          (i32.lt_s (i32.rem_s (local.get $n) (local.get $d)) (i32.const 1))
          (then
            (i32.const 0)
          )
          (else
            (call $is_prime_helper
              (local.get $n)
              (i32.add (local.get $d) (i32.const 1))
            )
          )
        )
      )
    )
  )

  (func $is_prime (export "is_prime") (param $n i32) (result i32)
    (if (result i32)
      (i32.lt_s (local.get $n) (i32.const 2))
      (then
        (i32.const 0)
      )
      (else
        (call $is_prime_helper
          (local.get $n)
          (i32.const 2)
        )
      )
    )
  )
)
