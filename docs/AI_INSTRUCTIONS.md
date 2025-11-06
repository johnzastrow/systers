**These are the standard rules to apply across all projects. Incorporate them into your instructions.**

**Create the following directory structure:**

- docs/ - for all documentation, including but not limited to all .md files except README
- tests/ - for all test materials
- images/ - for all graphics, icons, and pictures

**Always create and maintain the following .md files in `docs/`**

- TODO.md
- CHANGELOG.md
- REQUIREMENTS.md


## Rules

1. Use mermaid diagrams throughout the documentation
2. Only include information that explains the project and how to get started quickly in the README. Put other instructions in dedicated and relevant .md files in the docs/ directory. Reference these supplemental files in the README
3. Always write and perform unit tests and store them in `tests/`
4. Always apply appropriate linters, checkers, and formatters to code after you implement a batch of changes.
5. Always ask me to install helpful utilities that you may use to make your work more efficient. For example, uv, ruff, and ty for python. Then provide instructions on how to enable them for your use
6. Always evaluate the plan and the code before and after writing them for security implications and explain what you find. Always implement secure practices and assume the code will be used in publicly exposed places.
5. Always create a single file or location to track a semantic version number for the application. Increment that version appropriately when code is changed, including for bug fixes and new features. Use that version number throughout the code, including the database schema, displayed when the code is run, and in the code files and tests.

## Coding guidelines

- Prioritize modularity, clean code organization, and efficient resource management.
- Use expressive variable names that convey intent (e.g., `is_ready`, `has_data`).
- Avoid code duplication; use functions and modules to encapsulate reusable logic.
- Write code with safety, concurrency, and performance in mind, embracing Rust's ownership and type system.

### Error Handling and Safety
- Embrace Rust's Result and Option types for error handling.
- Use `?` operator to propagate errors in async functions.
- Implement custom error types using `thiserror` or `anyhow` for more descriptive errors.
- Handle errors and edge cases early, returning errors where appropriate.
- Use `.await` responsibly, ensuring safe points for context switching.

### Key Conventions
1. Structure the application into modules: separate concerns like networking, database, and business logic.
2. Ensure code is well-documented with inline comments and Rustdoc.

