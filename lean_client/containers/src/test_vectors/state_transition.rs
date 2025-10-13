#[cfg(test)]
mod tests {
    use super::super::runner::TestRunner;

    #[test]
    fn run_basic_state_transition_tests() {
        let test_path = "test_vectors/state_transition/basic.yaml";
        if std::path::Path::new(test_path).exists() {
            TestRunner::run_state_transition_tests(test_path)
                .expect("State transition tests should pass");
        } else {
            println!("Test vector file not found, skipping: {}", test_path);
        }
    }
}