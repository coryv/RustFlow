---
trigger: manual
---

- **Preserve functionality:** The most important rule is not to change what the code does externally. You should not add new features or fix bugs during a refactoring, but rather improve the underlying structure.
- **Be incremental:** Refactor in small, manageable steps to reduce the risk of introducing bugs. This allows for easier validation and rollback if something goes wrong.
- **Test frequently:** Use automated tests to verify that each small change hasn't broken the existing functionality. If a test fails, fix the error or roll back the change.
- **Improve readability:** Make the code easier for other developers (or your future self) to understand. This can involve renaming variables, extracting methods, or simplifying complex logic.
- **Reduce redundancy:** Eliminate duplicate code by consolidating it into reusable units. This makes the codebase more maintainable and can improve performance.
- **Separate concerns:** Break down large functions or classes into smaller, more focused units that each have a single responsibility. This simplifies debugging and testing.