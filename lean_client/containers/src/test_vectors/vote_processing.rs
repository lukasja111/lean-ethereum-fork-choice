#[cfg(test)]
mod tests {
    use super::super::runner::TestRunner;

    #[test]
    fn run_justification_tests() {
        let test_path = "test_vectors/vote_processing/justification.yaml";
        if std::path::Path::new(test_path).exists() {
            TestRunner::run_vote_processing_tests(test_path)
                .expect("Vote processing tests should pass");
        } else {
            println!("Test vector file not found, skipping: {}", test_path);
        }
    }
}
