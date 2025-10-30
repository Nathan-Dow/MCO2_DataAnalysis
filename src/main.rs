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
    approved_budget: f64,
    contract_cost: f64,
    start_date: NaiveDate,
    actual_completion_date: NaiveDate,
    funding_year: i32,
}

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
        savings.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_savings = if savings.is_empty() {
            0.0
        } else if savings.len() % 2 == 1 {
            savings[savings.len() / 2]
        } else {
            let mid = savings.len() / 2;
            (savings[mid - 1] + savings[mid]) / 2.0
        };

        // Compute completion delays
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

    // Display Report 1
    println!();
    println!("Report 1: Regional Flood Mitigation Efficiency Summary");
    println!("(Aggregated by Region & MainIsland; 2021–2023 Projects)");
    println!();
    println!(
        "| {:<38} | {:<13} | {:>15} | {:>15} | {:>15} | {:>12} | {:>16} |",
        "Region", "MainIsland", "TotalBudget", "MedianSavings", "AvgDelayDays", "Delay>30Pct", "EfficiencyScore"
    );
    println!("{}", "-".repeat(160));

    fn format_comma_float(val: f64) -> String {
        use num_format::{Locale, ToFormattedString};
        let whole = val.trunc() as i64;
        let fraction = (val.fract().abs() * 100.0).round() as u8;
        format!("{}.{:02}", whole.to_formatted_string(&Locale::en), fraction)
    }

    for r in &rows {
        println!(
            "| {:<38} | {:<13} | {:>15} | {:>15} | {:>15.2} | {:>12.1} | {:>16.2} |",
            r.region,
            r.main_island,
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

    Ok(())
}

