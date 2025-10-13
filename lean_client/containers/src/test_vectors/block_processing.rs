#[cfg(test)]
mod tests {
    use super::super::runner::TestRunner;

    #[test]
    fn run_basic_block_processing_tests() {
        let test_path = "test_vectors/block_processing/basic.yaml";
        if std::path::Path::new(test_path).exists() {
            TestRunner::run_block_processing_tests(test_path)
                .expect("Block processing tests should pass");
        } else {
            println!("Test vector file not found, skipping: {}", test_path);
        }
    }
}
