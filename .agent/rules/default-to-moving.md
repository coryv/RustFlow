---
trigger: model_decision
description: When developing anything in Rust
---

You are an expert Rust developer.

### 1. Don't Fight the Borrow Checker
* **Default to moving:** Let functions consume data if they don't need to return it.
* **Clone if necessary:** When prototyping, `.clone()` is acceptable to satisfy the borrow checker. Optimize later.
* **Rethink ownership:** If two things need to own the same data, use `Rc` (Reference Counted) or `Arc` (Atomic Reference Counted), not complex lifetime annotations.

### 2. Make Illegal States Unrepresentable
Use Rust's strong type system to ensure your code literally cannot compile if the logic is wrong.
* **Use Enums:** unlike other languages, Rust enums can hold data. Use them to model state machines (e.g., `State::Loading`, `State::Loaded(Data)`).
* **Newtype Pattern:** Don't just pass `String` or `i32` around. Wrap them in a tuple struct (e.g., `struct UserId(i32)`) so you don't accidentally pass a `PostId` into a function expecting a `UserId`.

### 3. Handle Errors, Don't Panic
Rust doesn't use exceptions; it uses `Result<T, E>`.
* **Avoid `.unwrap()`:** This will crash your program if it fails. Only use it in quick prototypes or tests.
* **Use `?`:** The question mark operator propagates errors up the stack cleanly.
* **Use `Option`:** If a value might not exist, use `Option<T>` rather than null checks.

### 4. Composition Over Inheritance
* **Use Traits:** Define shared behavior via Traits (interfaces).
* **Composition:** Build complex objects by including other structs inside them.

### 5. Lean on the Tooling
Rust has some of the best built-in tooling in the industry.
* **`cargo fmt`:** Run this automatically. Don't argue about code style.
* **`cargo clippy`:** This is a linter that will catch common mistakes and non-idiomatic code. **Treat clippy warnings as errors.**
