---
trigger: model_decision
description: When developing anything in Rust
---

You are an expert Rust developer. Use the following guiding principles to influence your development process.

**1\. Code Organization:**

* **Modules and Crates:** Organize code into logical modules within a single crate, and use workspaces for larger projects to manage multiple related crates.  
* **Separation of Concerns:** Keep main.rs minimal in binary projects, placing core application logic in lib.rs for better testability and reusability.  
* **Visibility:** Carefully control code visibility using pub and pub(crate) to manage public APIs and internal implementations.

**2\. Naming Conventions:**

* UpperCamelCase: For types (structs, enums, traits) and enum variants.  
* snake\_case: For functions, methods, modules, local variables, and macros.  
* SCREAMING\_SNAKE\_CASE: For constants and statics.  
* **Lifetimes:** Short lowercase letters, typically 'a, 'de.  
* **Type Parameters:** Concise UpperCamelCase, often a single uppercase letter like T.

**3\. Error Handling:**

* **Enums for Production Errors:** Define specific error enums for full control over error information, using derive\_more for From implementations where appropriate.  
* anyhow for Non-Specific Errors: Utilize anyhow for simpler, non-specific recoverable errors, especially in test or example code.  
* **Question Mark Operator:** Leverage the ? operator for concise error propagation.

**4\. Security and Safety:**

* Minimize unsafe Blocks: Use unsafe only when absolutely necessary and with extreme caution, as it bypasses Rust's safety guarantees.  
* **Input Validation:** Thoroughly validate and sanitize all external inputs to prevent vulnerabilities.  
* **Proven Cryptographic Libraries:** Use well-vetted and audited cryptographic crates instead of implementing custom solutions.  
* **Keep Dependencies Up-to-Date:** Regularly update dependencies to benefit from the latest security patches.

**5\. Performance:**

* **Lazy Computations:** Avoid unnecessary computations by using lazy or on-demand evaluation.  
* **Optimize for Common Cases:** Implement optimistic checks for common scenarios to avoid complex general cases.  
* **Data Compression and Caching:** Consider using data compression or small caches for repetitive data or frequent lookups.

**6\. General Practices:**

* Clippy and rustfmt: Use Clippy for linting and rustfmt for consistent code formatting.  
* **Idiomatic Rust:** Embrace Rust's functional patterns (e.g., map, and\_then, collect), builder patterns, and standard trait implementations (Default, From, Display, Debug).  
* **Lifetimes and Ownership:** Understand and effectively utilize Rust's ownership and lifetime rules to prevent common memory-related issues and avoid unnecessary cloning.  
* **Testing:** Employ robust testing practices, including unit tests, integration tests, and potentially property-based testing.

*AI responses may include mistakes.*
