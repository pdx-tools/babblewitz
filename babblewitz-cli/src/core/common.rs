/// Helper function to print table headers with consistent formatting
pub fn print_table_header(
    implementation_width: usize,
    columns: &[impl AsRef<str>],
    column_width: usize,
) {
    print!(
        "{:<width$} ",
        "Implementation",
        width = implementation_width
    );
    for column in columns {
        print!(
            "{:>width$} ",
            column.as_ref().to_uppercase(),
            width = column_width
        );
    }
    println!();

    // Print separator line
    print!("{} ", "-".repeat(implementation_width));
    for _ in columns {
        print!("{} ", "-".repeat(column_width));
    }
    println!();
}

/// Calculate maximum implementation name width for table formatting
pub fn calculate_impl_width(implementations: &[String]) -> usize {
    implementations
        .iter()
        .map(|impl_name| impl_name.len())
        .fold("Implementation".len(), |max, len| max.max(len))
        .max(15)
}
