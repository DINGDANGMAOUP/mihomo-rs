use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::ExecutableCommand;
use std::io::stdout;
use unicode_width::UnicodeWidthStr;

pub fn print_success(msg: &str) {
    let mut stdout = stdout();
    let _ = stdout.execute(SetForegroundColor(Color::Green));
    let _ = stdout.execute(Print("✓ "));
    let _ = stdout.execute(ResetColor);
    println!("{}", msg);
}

pub fn print_error(msg: &str) {
    let mut stdout = stdout();
    let _ = stdout.execute(SetForegroundColor(Color::Red));
    let _ = stdout.execute(Print("✗ "));
    let _ = stdout.execute(ResetColor);
    eprintln!("{}", msg);
}

pub fn print_info(msg: &str) {
    let mut stdout = stdout();
    let _ = stdout.execute(SetForegroundColor(Color::Blue));
    let _ = stdout.execute(Print("ℹ "));
    let _ = stdout.execute(ResetColor);
    println!("{}", msg);
}

pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        return;
    }

    let mut col_widths = headers.iter().map(|h| display_width(h)).collect::<Vec<_>>();

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(display_width(cell));
            }
        }
    }

    print!("│ ");
    for (i, header) in headers.iter().enumerate() {
        print_padded(header, col_widths[i]);
        if i < headers.len() - 1 {
            print!(" │ ");
        }
    }
    println!(" │");

    print!("├");
    for (i, width) in col_widths.iter().enumerate() {
        print!("{}", "─".repeat(width + 2));
        if i < col_widths.len() - 1 {
            print!("┼");
        }
    }
    println!("┤");

    for row in rows {
        print!("│ ");
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                print_padded(cell, col_widths[i]);
                if i < row.len() - 1 {
                    print!(" │ ");
                }
            }
        }
        println!(" │");
    }
}

fn display_width(input: &str) -> usize {
    UnicodeWidthStr::width(input)
}

fn print_padded(input: &str, width: usize) {
    print!("{}", input);
    let current = display_width(input);
    if width > current {
        print!("{}", " ".repeat(width - current));
    }
}

#[cfg(test)]
mod tests {
    use super::display_width;

    #[test]
    fn test_display_width_mixed_language() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("测试"), 4);
        assert_eq!(display_width("a测b"), 4);
    }
}
