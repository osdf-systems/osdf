use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context};
use osdf_core::{verify_package_bytes, PackageContainer, VerificationReport, VerificationStatus};

const CONTENT_PATH: &str = "content/document.json";
const TAMPER_CONTENT_BYTE: usize = 32;

pub fn run_safety_demo(path: &Path, write_assets: Option<&Path>) -> anyhow::Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read demo fixture `{}`", path.display()))?;

    println!("OSDF mathematical safety demo");
    println!("Fixture: {}\n", path.display());

    let pass_start = Instant::now();
    let pass_report = verify_package_bytes(&bytes);
    let pass_ms = pass_start.elapsed().as_secs_f64() * 1000.0;

    print_demo_panel("Authentic document", &pass_report, pass_ms);

    let tampered = tamper_content_byte(&bytes)?;
    let fail_start = Instant::now();
    let fail_report = verify_package_bytes(&tampered);
    let fail_ms = fail_start.elapsed().as_secs_f64() * 1000.0;

    println!();
    println!("Tamper: flipped 1 byte inside signed payload (offset {TAMPER_CONTENT_BYTE} in `{CONTENT_PATH}`)");
    println!();

    print_demo_panel("After 1-byte tamper", &fail_report, fail_ms);

    if let Some(dir) = write_assets {
        write_readme_assets(dir, &pass_report, pass_ms, &fail_report, fail_ms)?;
        eprintln!("\nWrote README assets to {}", dir.display());
    }

    if pass_report.overall != VerificationStatus::Pass {
        bail!("expected PASS on authentic fixture");
    }
    if fail_report.overall != VerificationStatus::Fail {
        bail!("expected FAIL after tamper");
    }

    Ok(())
}

fn tamper_content_byte(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut container = PackageContainer::read_from_bytes(bytes)?;
    let content = container
        .get(CONTENT_PATH)
        .with_context(|| format!("fixture missing `{CONTENT_PATH}`"))?
        .to_vec();
    if content.len() <= TAMPER_CONTENT_BYTE {
        bail!("content object too small for demo tamper");
    }
    let mut tampered = content;
    tampered[TAMPER_CONTENT_BYTE] ^= 0xFF;
    container.insert(CONTENT_PATH, tampered)?;
    container.to_bytes().context("repack tampered fixture")
}

fn print_demo_panel(title: &str, report: &VerificationReport, elapsed_ms: f64) {
    let status = report.overall.as_str();
    println!("┌─ {title} ─────────────────────────────────────────");
    println!("│ Overall: {status}   ({elapsed_ms:.2} ms)");
    println!("│");

    for section in &report.sections {
        let section_pass = section
            .checks
            .iter()
            .all(|c| c.status != VerificationStatus::Fail);
        let marker = if section_pass { "PASS" } else { "FAIL" };
        println!("│ [{marker}] {}", section.title);

        for check in &section.checks {
            if check.status == VerificationStatus::Info {
                continue;
            }
            println!("│       [{}] {}", check.status.as_str(), check.label);
        }
    }

    for finding in &report.findings {
        if finding.severity == osdf_core::Severity::Info {
            continue;
        }
        let tag = match finding.severity {
            osdf_core::Severity::Fail => "FAIL",
            osdf_core::Severity::Warning => "WARN",
            osdf_core::Severity::Info => "INFO",
        };
        println!("│");
        println!("│ [{tag}] {}", finding.summary);
        println!("│       ({})", finding.code);
    }

    if let Some(document_id) = &report.document_id {
        println!("│");
        println!("│ Document: {document_id}");
    }
    if let Some(revision) = report.revision {
        println!("│ Revision: {revision}   Signatures: {}", report.signature_count);
    }

    println!("└────────────────────────────────────────────────────────");
}

fn write_readme_assets(
    dir: &Path,
    pass_report: &VerificationReport,
    pass_ms: f64,
    fail_report: &VerificationReport,
    fail_ms: f64,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;

    let pass_lines = panel_lines("Authentic document", pass_report, pass_ms);
    let fail_lines = panel_lines("After 1-byte tamper", fail_report, fail_ms);

    std::fs::write(
        dir.join("demo-verify-pass.svg"),
        terminal_svg(&pass_lines, VerificationStatus::Pass),
    )?;
    std::fs::write(
        dir.join("demo-verify-fail.svg"),
        terminal_svg(&fail_lines, VerificationStatus::Fail),
    )?;

    Ok(())
}

fn panel_lines(title: &str, report: &VerificationReport, elapsed_ms: f64) -> Vec<String> {
    let mut lines = vec![
        title.to_string(),
        format!("Overall: {}   ({elapsed_ms:.2} ms)", report.overall.as_str()),
        String::new(),
    ];

    for section in &report.sections {
        let section_pass = section
            .checks
            .iter()
            .all(|c| c.status != VerificationStatus::Fail);
        let marker = if section_pass { "PASS" } else { "FAIL" };
        lines.push(format!("[{marker}] {}", section.title));
        for check in &section.checks {
            if check.status == VerificationStatus::Info {
                continue;
            }
            lines.push(format!("      [{}] {}", check.status.as_str(), check.label));
        }
    }

    for finding in &report.findings {
        if finding.severity == osdf_core::Severity::Info {
            continue;
        }
        lines.push(String::new());
        lines.push(format!("[FAIL] {}", finding.summary));
        lines.push(format!("      ({})", finding.code));
    }

    if let Some(document_id) = &report.document_id {
        lines.push(String::new());
        lines.push(format!("Document: {document_id}"));
    }
    if let Some(revision) = report.revision {
        lines.push(format!(
            "Revision: {revision}   Signatures: {}",
            report.signature_count
        ));
    }

    lines
}

fn terminal_svg(lines: &[String], overall: VerificationStatus) -> String {
    let accent = match overall {
        VerificationStatus::Pass => "#3fb950",
        VerificationStatus::Warning => "#d29922",
        VerificationStatus::Fail => "#f85149",
        VerificationStatus::Info => "#8b949e",
    };

    let line_height = 18;
    let padding = 16;
    let width = 720;
    let height = padding * 2 + lines.len() * line_height + 8;

    let mut text_elements = String::new();
    for (index, line) in lines.iter().enumerate() {
        let y = padding + 14 + index * line_height;
        let escaped = escape_xml(line);
        let fill = if line.starts_with("Overall: PASS") {
            "#3fb950"
        } else if line.starts_with("Overall: FAIL") {
            "#f85149"
        } else if line.starts_with("[PASS]") {
            "#3fb950"
        } else if line.starts_with("[FAIL]") {
            "#f85149"
        } else {
            "#e6edf3"
        };
        text_elements.push_str(&format!(
            "<text x=\"{padding}\" y=\"{y}\" fill=\"{fill}\" font-family=\"Consolas, 'Courier New', monospace\" font-size=\"13\">{escaped}</text>\n"
        ));
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="100%" height="100%" rx="8" fill="#0d1117"/>
  <rect x="1" y="1" width="{w2}" height="{h2}" rx="8" fill="none" stroke="{accent}" stroke-width="2"/>
{text_elements}
</svg>"##,
        width = width,
        height = height,
        w2 = width - 2,
        h2 = height - 2,
        accent = accent,
        text_elements = text_elements,
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
