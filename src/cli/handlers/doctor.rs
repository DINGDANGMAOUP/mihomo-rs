use crate::cli::{print_info, print_table, DoctorAction};
use crate::doctor::{
    explain_check, fix_doctor, list_checks, run_doctor, DoctorRunOptions, DoctorStatus,
};

pub async fn handle_doctor(action: DoctorAction) -> anyhow::Result<i32> {
    match action {
        DoctorAction::Run { only, json } => {
            let report = run_doctor(DoctorRunOptions { only }).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_doctor_report(&report);
            }

            return Ok(if report.has_failures() { 1 } else { 0 });
        }
        DoctorAction::List => {
            let rows = list_checks()
                .into_iter()
                .map(|check| {
                    vec![
                        check.id.to_string(),
                        check.category.to_string(),
                        if check.fixable {
                            "yes".to_string()
                        } else {
                            "no".to_string()
                        },
                        if check.default_enabled {
                            "yes".to_string()
                        } else {
                            "no".to_string()
                        },
                        check.summary.to_string(),
                    ]
                })
                .collect();
            print_table(&["ID", "Category", "Fixable", "Default", "Summary"], rows);
        }
        DoctorAction::Fix { only, json } => {
            let report = fix_doctor(DoctorRunOptions { only }).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.fixes.is_empty() {
                print_info("No safe doctor fixes matched the filter");
            } else {
                let rows = report
                    .fixes
                    .iter()
                    .map(|fix| vec![fix.id.clone(), fix.summary.clone()])
                    .collect();
                print_table(&["Check", "Applied Fix"], rows);
            }
        }
        DoctorAction::Explain { check_id } => {
            let info = explain_check(&check_id)?;
            print_info(info.summary);
            println!("id: {}", info.id);
            println!("category: {}", info.category);
            println!("fixable: {}", if info.fixable { "yes" } else { "no" });
            println!(
                "default: {}",
                if info.default_enabled { "yes" } else { "no" }
            );
            println!("why: {}", info.why);
            println!("fail means: {}", info.fail_means);
            println!("hint: {}", info.hint);
        }
    }

    Ok(0)
}

fn print_doctor_report(report: &crate::doctor::DoctorReport) {
    let rows: Vec<Vec<String>> = report
        .checks
        .iter()
        .map(|check| {
            vec![
                check.status.as_str().to_string(),
                check.id.clone(),
                check.summary.clone(),
                check.hint.clone().unwrap_or_default(),
            ]
        })
        .collect();

    if rows.is_empty() {
        print_info("No doctor checks matched the filter");
        return;
    }

    print_table(&["Status", "Check", "Summary", "Hint"], rows);
    let summary = format!(
        "doctor: {} pass, {} warn, {} fail, {} skip",
        report.count_by_status(DoctorStatus::Pass),
        report.count_by_status(DoctorStatus::Warn),
        report.count_by_status(DoctorStatus::Fail),
        report.count_by_status(DoctorStatus::Skip),
    );

    if report.has_failures() {
        eprintln!("{}", summary);
    } else {
        println!("{}", summary);
    }
}
