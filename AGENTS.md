the source code for the project is at crates/tunfetch

the project follows a TDD (Test Driven Development) approach for the tunfetch rust crate. before implementing anything you have to first think about the tests and then about the implementation. 

the structure of the codebase should create a thin layer which setups the wasm bindings and leave the core logic of the codebase wasm independent and unit testable

after modifying the codebase always run the tests with `cargo test` and make sure all tests pass.

read README.md to understand what the project is about.