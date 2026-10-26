use std::path::Path;

use crate::hygiene;

pub fn run(limit: usize, complexity: usize) {
    let root = Path::new(".");
    let length_warnings = hygiene::check_file_lengths(root, limit);
    for w in &length_warnings {
        println!(
            "warning: {} has {} lines, exceeding the {}-line soft limit",
            w.path.display(),
            w.lines,
            limit
        );
    }

    let complexity_warnings = hygiene::check_complexity(root, complexity);
    for cw in &complexity_warnings {
        println!(
            "warning: {}:{} '{}' has cyclomatic complexity {}, exceeding the {} limit (max nesting: {})",
            cw.path.display(),
            cw.line,
            cw.fn_name,
            cw.complexity,
            complexity,
            cw.max_nesting
        );
    }

    let test_warnings = hygiene::check_test_placement(root);
    for tw in &test_warnings {
        println!(
            "warning: [{}] is an integration test suite under src/; move it to tests/",
            tw.display()
        );
    }

    println!(
        "\nFile length warnings: {}. Complexity warnings: {}. Placement warnings: {}.",
        length_warnings.len(),
        complexity_warnings.len(),
        test_warnings.len()
    );
}
