use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::ExecutableCommand;
use std::io::stdout;

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

    let mut col_widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    print!("│ ");
    for (i, header) in headers.iter().enumerate() {
        print!("{:width$}", header, width = col_widths[i]);
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
                print!("{:width$}", cell, width = col_widths[i]);
                if i < row.len() - 1 {
                    print!(" │ ");
                }
            }
        }
        println!(" │");
    }
}
