use typst::syntax::Source;

fn main() {
    let source = Source::detached("$sum_(a)^(b)$");
    let text = source.text();
    println!("Source text: {:?}", text);
    println!("Text len: {}", text.len());
    
    // Print character positions
    for (i, c) in text.char_indices() {
        println!("  offset {}: '{}'", i, c);
    }
    
    // Test intercept_math_deletion for each position inside `sum`
    // s is at offset 1, u at 2, m at 3
    for start in 1..=3 {
        let end = start + 1;
        println!("\nTesting intercept_math_deletion({}..{}), deleting '{}':", start, end, &text[start..end]);
        let result = typst_writer::geometry::intercept_math_deletion(&source, start..end);
        println!("  Result: {:?}", result);
        if let Some(ref range) = result {
            println!("  Selected text: {:?}", &text[range.clone()]);
        }
    }
    
    // Also test positions around the attachment operators
    for start in 4..=11 {
        let end = start + 1;
        if end <= text.len() {
            println!("\nTesting intercept_math_deletion({}..{}), deleting '{}':", start, end, &text[start..end]);
            let result = typst_writer::geometry::intercept_math_deletion(&source, start..end);
            println!("  Result: {:?}", result);
            if let Some(ref range) = result {
                println!("  Selected text: {:?}", &text[range.clone()]);
            }
        }
    }
}
