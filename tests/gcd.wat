;; Euclidean algorithm for the greatest common divisor: https://en.wikipedia.org/wiki/Euclidean_algorithm#Implementations
(module
  (func $gcd (export "gcd") (param $a i32) (param $b i32) (result i32)
    (if (result i32)
      (i32.lt_s (local.get $b) (i32.const 1))
      (then
        (local.get $a)
      )
      (else
        (call $gcd
          (local.get $b)
          (i32.rem_s (local.get $a) (local.get $b))
        )
      )
    )
  )
)
