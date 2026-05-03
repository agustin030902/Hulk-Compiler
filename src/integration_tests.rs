// Integration tests for conditional expressions (if/else/elif)

#[cfg(test)]
mod integration_tests {
    use std::process::Command;

    fn run_hulk_example(filename: &str) -> String {
        let output = Command::new("cargo")
            .args(&["run", "--", "run", "--input", &format!("examples/{}", filename)])
            .output()
            .expect("Failed to execute cargo run");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn test_if_else_simple_example() {
        let output = run_hulk_example("if_else_simple.hulk");
        assert!(
            output.contains("Greater"),
            "Expected 'Greater' in output, got: {}",
            output
        );
    }

    #[test]
    fn test_if_else_expr_example() {
        let output = run_hulk_example("if_else_expr.hulk");
        assert!(
            output.contains("greater"),
            "Expected 'greater' in output, got: {}",
            output
        );
    }

    #[test]
    fn test_if_else_block_example() {
        let output = run_hulk_example("if_else_block.hulk");
        assert!(
            output.contains("42") && output.contains("Greater"),
            "Expected '42' and 'Greater' in output, got: {}",
            output
        );
    }

    #[test]
    fn test_if_elif_else_example() {
        let output = run_hulk_example("if_elif_else.hulk");
        assert!(
            output.contains("Other"),
            "Expected 'Other' in output, got: {}",
            output
        );
    }

    #[test]
    fn test_if_multiple_elif_example() {
        let output = run_hulk_example("if_multiple_elif.hulk");
        assert!(
            output.contains("Single digit"),
            "Expected 'Single digit' in output, got: {}",
            output
        );
    }
}
