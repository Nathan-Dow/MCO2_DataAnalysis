use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Mutex;
use chrono::{Datelike, NaiveDate};
use once_cell::sync::Lazy;
use num_format::{Locale, ToFormattedString};

static APP_STATE: Lazy<Mutex<AppState>> = Lazy::new(|| Mutex::new(AppState::default()));

#[derive(Default)]
struct AppState {
    projects: Vec<Project>,
}

#[derive(Clone)]
struct Project {
    region: String,
    main_island: String,
    contractor: String,
    approved_budget: f64,
    contract_cost: f64,
    start_date: NaiveDate,
    actual_completion_date: NaiveDate,
    funding_year: i32,
}

// struct Project {
//     main_island: String,
//     region: String,
//     province: String,
//     legislative_district: String,
//     municipality: String,
//     district_engineering_office: String,
//     project_id: String,
//     project_name: String,
//     work_type: String,
//     approved_budget: f64,
//     contract_cost: f64,
//     start_date: NaiveDate,
//     actual_completion_date: NaiveDate,
//     contractor_name: String,
//     contractor_count: i32,
//     funding_year: i32,
// }




fn main() -> Result<(), Box<dyn Error>> {
    loop {
        println!("Select Language Implementation:");
        println!("[1] Load the file");
        println!("[2] Generate Reports");
        print!("Enter Choice: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        match choice.trim() {
            "1" => load_and_process_file()?,
            "2" => generate_reports()?,
            _ => println!("Invalid choice. Please try again."),
        }
        println!();
    }
}

fn load_and_process_file() -> Result<(), Box<dyn Error>> {
    print!("Enter CSV filename: ");
    io::stdout().flush().unwrap();
    let mut filename = String::new();
    io::stdin().read_line(&mut filename)?;
    let filename = filename.trim();

    let mut rdr = csv::Reader::from_path(filename)?;
    let headers = rdr.headers()?.clone();
    let mut total_rows = 0;
    let mut filtered_rows = 0;
    let mut error_count = 0;

    // Indexes for efficiency
    let funding_year_idx = headers.iter().position(|h| h == "FundingYear");
    let region_idx = headers.iter().position(|h| h == "Region");
    let main_island_idx = headers.iter().position(|h| h == "MainIsland");
    let contractor_idx = headers.iter().position(|h| h == "Contractor");
    let approved_budget_idx = headers.iter().position(|h| h == "ApprovedBudgetForContract");
    let contract_cost_idx = headers.iter().position(|h| h == "ContractCost");
    let start_date_idx = headers.iter().position(|h| h == "StartDate");
    let actual_completion_idx = headers.iter().position(|h| h == "ActualCompletionDate");
    
    for result in rdr.records() {
        total_rows += 1;
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Row {}: CSV parse error: {}", total_rows, e);
                error_count += 1;
                continue;
            }
        };
        // FundingYear validation and filter
        let fy = funding_year_idx.and_then(|i| record.get(i));
        let fy_num = match fy.and_then(|f| f.parse::<i32>().ok()) {
            Some(y) if y >= 2021 && y <= 2023 => y,
            Some(_) => continue,
            None => {
                eprintln!("Row {}: Invalid FundingYear: {:?}", total_rows, fy);
                error_count += 1;
                continue;
            },
        };

        let region = match region_idx.and_then(|i| record.get(i)) {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => { error_count += 1; continue; }
        };
        let main_island = match main_island_idx.and_then(|i| record.get(i)) {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => { error_count += 1; continue; }
        };

        let contractor = match contractor_idx.and_then(|i| record.get(i)) {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => { error_count += 1; continue; }
        };
        let approved_budget = match approved_budget_idx.and_then(|i| record.get(i)).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => { error_count += 1; continue; }
        };
        let contract_cost = match contract_cost_idx.and_then(|i| record.get(i)).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => { error_count += 1; continue; }
        };
        let start_date = match start_date_idx.and_then(|i| record.get(i)).and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok()) {
            Some(d) => d,
            None => { error_count += 1; continue; }
        };
        let actual_completion_date = match actual_completion_idx.and_then(|i| record.get(i)).and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok()) {
            Some(d) => d,
            None => { error_count += 1; continue; }
        };

        filtered_rows += 1;
        let mut state = APP_STATE.lock().unwrap();
        state.projects.push(Project {
            region,
            main_island,
            contractor,
            approved_budget,
            contract_cost,
            start_date,
            actual_completion_date,
            funding_year: fy_num,
        });
    }
    println!("Processing dataset... ({} rows loaded, {} filtered for 2021-2023)", total_rows, filtered_rows);
    if error_count > 0 {
        println!("{} parse/validation errors encountered.", error_count);
    }
    Ok(())
}

fn generate_reports() -> Result<(), Box<dyn Error>> {
    let projects = {
        let state = APP_STATE.lock().unwrap();
        state.projects.clone()
    };
    if projects.is_empty() {
        println!("No data loaded. Please choose [1] Load the file first.");
        return Ok(());
    }

    println!("Generating reports...");

    // Group by (Region, MainIsland)
    let mut grouped: HashMap<(String, String), Vec<&Project>> = HashMap::new();
    for p in &projects {
        grouped.entry((p.region.clone(), p.main_island.clone()))
            .or_default()
            .push(p);
    }

    // Define Row (make sure this isn't defined elsewhere already)
    #[derive(Clone)]
    struct Row {
        region: String,
        main_island: String,
        total_budget: f64,
        median_savings: f64,
        avg_delay: f64,
        delay_over30_pct: f64,
        efficiency_score: f64,
    }

    let mut rows: Vec<Row> = Vec::new();
    const DELAY_THRESHOLD_DAYS: i64 = 30;

    for ((region, main_island), items) in grouped {
        let total_budget: f64 = items.iter().map(|p| p.approved_budget).sum();

        // Compute savings (ApprovedBudgetForContract - ContractCost)
        let mut savings: Vec<f64> = items.iter().map(|p| p.approved_budget - p.contract_cost).collect();
        // Remove any NaN just in case (defensive)
        savings.retain(|v| !v.is_nan());
        savings.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_savings = if savings.is_empty() {
            0.0
        } else if savings.len() % 2 == 1 {
            savings[savings.len() / 2]
        } else {
            let mid = savings.len() / 2;
            (savings[mid - 1] + savings[mid]) / 2.0
        };

        // Compute completion delays (days)
        let delays: Vec<i64> = items.iter().map(|p| {
            let d = (p.actual_completion_date - p.start_date).num_days();
            if d < 0 { 0 } else { d }
        }).collect();

        let avg_delay = if delays.is_empty() { 0.0 } else { (delays.iter().sum::<i64>() as f64) / (delays.len() as f64) };
        let delay_over30_count = delays.iter().filter(|d| **d > DELAY_THRESHOLD_DAYS).count();
        let delay_over30_pct = if delays.is_empty() { 0.0 } else { (delay_over30_count as f64) * 100.0 / (delays.len() as f64) };

        // Compute efficiency score = (median_savings / avg_delay) * 100
        let raw_efficiency = if avg_delay > 0.0 {
            (median_savings / avg_delay) * 100.0
        } else {
            0.0
        };

        rows.push(Row {
            region,
            main_island,
            total_budget,
            median_savings,
            avg_delay,
            delay_over30_pct,
            efficiency_score: raw_efficiency,
        });
    }


    // Normalize efficiency scores to 0–100 range
    if let (Some(min), Some(max)) = (
        rows.iter().map(|r| r.efficiency_score).reduce(f64::min),
        rows.iter().map(|r| r.efficiency_score).reduce(f64::max),
    ) {
        for r in &mut rows {
            if max > min {
                r.efficiency_score = ((r.efficiency_score - min) / (max - min)) * 100.0;
            } else {
                r.efficiency_score = 100.0; // all same values
            }
        }
    }

    // Sort descending by EfficiencyScore
    rows.sort_by(|a, b| b.efficiency_score.partial_cmp(&a.efficiency_score).unwrap());

    use num_format::{Locale, ToFormattedString};

fn format_comma_float(val: f64) -> String {
    // Handles negatives and formats with commas + 2 decimal places
    let sign = if val.is_sign_negative() { "-" } else { "" };
    let abs_val = val.abs();
    let whole = abs_val.trunc() as i64;
    let fraction = (abs_val.fract() * 100.0).round() as u8;
    format!("{}{}.{:02}", sign, whole.to_formatted_string(&Locale::en), fraction)
}


    // Display Report 1
    println!();
    println!("Report 1: Regional Flood Mitigation Efficiency Summary");
    println!("(Aggregated by Region & MainIsland; 2021–2023 Projects)");
    println!();

    // Header with fixed widths
    println!(
        "| {:<40} | {:<10} | {:>18} | {:>15} | {:>13} | {:>12} | {:>17} |",
        "Region", "MainIsland", "TotalBudget", "MedianSavings", "AvgDelayDays", "Delay>30Pct", "EfficiencyScore"
    );
    println!("{}", "-".repeat(146));

    // Single loop: print each row once
    for r in &rows {
        println!(
            "| {:<40} | {:<10} | {:>18} | {:>15} | {:>13.2} | {:>12.1} | {:>17.2} |",
            r.region.trim(),
            r.main_island.trim(),
            format_comma_float(r.total_budget),
            format_comma_float(r.median_savings),
            r.avg_delay,
            r.delay_over30_pct,
            r.efficiency_score
        );
    }

    println!();
    println!("Full table exported to report_1_regional_summary.csv");



    // Export CSV (sorted)
    let mut wtr = csv::Writer::from_path("report_1_regional_summary.csv")?;
    wtr.write_record([
        "Region",
        "MainIsland",
        "TotalBudget",
        "MedianSavings",
        "AvgDelayDays",
        "DelayOver30Pct",
        "EfficiencyScore",
    ])?;
    for r in rows {
        wtr.write_record(&[
            r.region,
            r.main_island,
            format!("{:.2}", r.total_budget),
            format!("{:.2}", r.median_savings),
            format!("{:.2}", r.avg_delay),
            format!("{:.1}", r.delay_over30_pct),
            format!("{:.2}", r.efficiency_score),
        ])?;
    }
    wtr.flush()?;

    // =============================
    // Report 2: Contractor Ranking
    // =============================
    println!();
    println!("Report 2: Top Contractors Performance Ranking");
    println!("(Top 15 by TotalCost, >=5 Projects)");
    println!();

    // Group by Contractor
    let mut contractor_group: HashMap<String, Vec<&Project>> = HashMap::new();
    for p in &projects {
        contractor_group.entry(p.contractor.clone()).or_default().push(p);
    }

    #[derive(Debug)]
    struct ContractorRow {
        contractor: String,
        total_cost: f64,
        num_projects: usize,
        avg_delay: f64,
        total_savings: f64,
        reliability_index: f64,
        risk_flag: String,
    }

    let mut contractor_rows: Vec<ContractorRow> = Vec::new();

    for (contractor, items) in contractor_group {
        if items.len() < 5 {
            continue;
        }

        let total_cost: f64 = items.iter().map(|p| p.contract_cost).sum();
        let total_savings: f64 = items.iter().map(|p| p.approved_budget - p.contract_cost).sum();

        let delays: Vec<i64> = items.iter()
            .map(|p| (p.actual_completion_date - p.start_date).num_days().max(0))
            .collect();

        let avg_delay = if delays.is_empty() {
            0.0
        } else {
            delays.iter().sum::<i64>() as f64 / delays.len() as f64
        };

        let mut reliability_index = (1.0 - (avg_delay / 90.0)) * (total_savings / total_cost) * 100.0;
        if reliability_index > 100.0 {
            reliability_index = 100.0;
        } else if reliability_index < 0.0 {
            reliability_index = 0.0;
        }

        let risk_flag = if reliability_index < 50.0 {
            "High Risk".to_string()
        } else {
            "Low Risk".to_string()
        };

        contractor_rows.push(ContractorRow {
            contractor,
            total_cost,
            num_projects: items.len(),
            avg_delay,
            total_savings,
            reliability_index,
            risk_flag,
        });
    }

    // Sort by descending total cost
    contractor_rows.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());

    // Keep top 15
    let top_rows: Vec<_> = contractor_rows.into_iter().take(5000).collect();

    // Print formatted table
    // Helper: truncate long contractor names for display
    fn truncate_name(name: &str, max_len: usize) -> String {
        if name.len() > max_len {
            format!("{}...", &name[..max_len - 3])
        } else {
            name.to_string()
        }
    }

    // Print formatted table
    println!(
        "| {:<4} | {:<45} | {:<18} | {:<12} | {:<10} | {:<16} | {:<18} | {:<10} |",
        "Rank", "Contractor", "TotalCost", "NumProjects", "AvgDelay", "TotalSavings", "ReliabilityIndex", "RiskFlag"
    );
    println!("{}", "-".repeat(165));

    for (i, r) in top_rows.iter().enumerate() {
        println!(
            "| {:<4} | {:<45} | {:>18} | {:>12} | {:>10.1} | {:>16} | {:>18.2} | {:<10} |",
            i + 1,
            truncate_name(&r.contractor, 45),
            format_comma_float(r.total_cost),
            r.num_projects,
            r.avg_delay,
            format_comma_float(r.total_savings),
            r.reliability_index,
            r.risk_flag
        );
    }


    println!();
    println!("Full table exported to report_2_contractor_ranking.csv");

    // Export CSV
    let mut wtr2 = csv::Writer::from_path("report_2_contractor_ranking.csv")?;
    wtr2.write_record([
        "Rank",
        "Contractor",
        "TotalCost",
        "NumProjects",
        "AvgDelay",
        "TotalSavings",
        "ReliabilityIndex",
        "RiskFlag",
    ])?;

    for (i, r) in top_rows.iter().enumerate() {
        wtr2.write_record(&[
            (i + 1).to_string(),
            r.contractor.clone(),
            format!("{:.2}", r.total_cost),
            r.num_projects.to_string(),
            format!("{:.2}", r.avg_delay),
            format!("{:.2}", r.total_savings),
            format!("{:.2}", r.reliability_index),
            r.risk_flag.clone(),
        ])?;
    }
    wtr2.flush()?;

    Ok(())
}

