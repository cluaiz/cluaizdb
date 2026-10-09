pub mod config;
pub mod error;
pub mod runner;

pub use config::SandboxConfig;
pub use error::SandboxError;
pub use runner::WasmGenomeRunner;

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::Val;

    #[test]
    fn test_wasm_scalar_addition() {
        let wat_src = r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )
        "#;

        let wasm_bytes = wat::parse_str(wat_src).expect("Failed to parse WAT");
        let runner = WasmGenomeRunner::new(SandboxConfig::default()).unwrap();
        let module = runner.compile(&wasm_bytes).unwrap();

        let results = runner
            .call_scalar(&module, "add", &[Val::I32(18), Val::I32(24)])
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].i32(), Some(42));
    }

    #[test]
    fn test_wasm_out_of_fuel_trap() {
        let wat_src = r#"
            (module
                (func (export "infinite_loop")
                    (loop
                        br 0
                    )
                )
            )
        "#;

        let wasm_bytes = wat::parse_str(wat_src).expect("Failed to parse WAT");
        // Tight fuel limit to trigger trap quickly
        let config = SandboxConfig::new(500, 1);
        let runner = WasmGenomeRunner::new(config).unwrap();
        let module = runner.compile(&wasm_bytes).unwrap();

        let result = runner.call_scalar(&module, "infinite_loop", &[]);
        assert!(matches!(result, Err(SandboxError::OutOfFuel)));
    }

    #[test]
    fn test_wasm_linear_memory_transform() {
        let wat_src = r#"
            (module
                (memory (export "memory") 1)
                (func (export "increment_bytes") (param $offset i32) (param $len i32) (result i32 i32)
                    (local $i i32)
                    (local.set $i (i32.const 0))
                    (block $done
                        (loop $loop
                            (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
                            (i32.store8
                                (i32.add (local.get $offset) (local.get $i))
                                (i32.add
                                    (i32.load8_u (i32.add (local.get $offset) (local.get $i)))
                                    (i32.const 1)
                                )
                            )
                            (local.set $i (i32.add (local.get $i) (i32.const 1)))
                            (br $loop)
                        )
                    )
                    (local.get $offset)
                    (local.get $len)
                )
            )
        "#;

        let wasm_bytes = wat::parse_str(wat_src).expect("Failed to parse WAT");
        let runner = WasmGenomeRunner::new(SandboxConfig::default()).unwrap();
        let module = runner.compile(&wasm_bytes).unwrap();

        let input_data = vec![10u8, 20u8, 30u8, 40u8];
        let output = runner
            .execute_memory_transform(&module, "increment_bytes", &input_data)
            .unwrap();

        assert_eq!(output, vec![11u8, 21u8, 31u8, 41u8]);
    }
}
