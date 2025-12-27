//! Conative Gating CLI
//!
//! Command-line interface for the policy oracle.
//!
//! # Overview
//!
//! Conative Gating implements SLM-as-Cerebellum architecture for LLM policy
//! enforcement. It provides deterministic rule checking via the Policy Oracle
//! and (in v2) neural evaluation via Small Language Models.
//!
//! # Dry Run Mode
//!
//! Most commands support `--dry-run` to preview actions without side effects.
//! Use `--verbose` for detailed operation logging.
//!
//! # Reversibility
//!
//! This tool is read-only by design. It analyzes but never modifies files.
//! All operations are safe to run repeatedly.

use clap::{Parser, Subcommand, ValueEnum};
use gating_contract::{
    AuditEntry, CategoryStats, ContractRunner, GatingRequest, RedTeamCategory, RedTeamSummary,
    RegressionBaseline, RegressionHarness, TestCase, TestHarness, Verdict,
};
use policy_oracle::{ActionType, DirectoryScanResult, Oracle, Policy, Proposal};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Output format for results
#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for machine processing
    Json,
    /// Compact single-line output
    Compact,
}

/// Verbosity level
#[derive(Debug, Clone, ValueEnum)]
enum Verbosity {
    /// Suppress all non-error output
    Quiet,
    /// Normal output
    Normal,
    /// Detailed output with rule explanations
    Verbose,
    /// Debug output including internal state
    Debug,
}

#[derive(Parser)]
#[command(name = "conative")]
#[command(author = "Jonathan D.A. Jewell <jonathan@hyperpolymath.org>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "SLM-as-Cerebellum for LLM Policy Enforcement")]
#[command(long_about = r#"
Conative Gating - Policy Enforcement for AI-Assisted Development

OVERVIEW
  Provides dual-layer policy checking for code proposals:
  1. Policy Oracle: Deterministic rule checking (forbidden languages, toolchain)
  2. SLM Evaluator: Neural "spirit of policy" evaluation (v2)

SAFETY
  This tool is READ-ONLY. It analyzes files but never modifies them.
  All operations are idempotent and safe to run repeatedly.

EXAMPLES
  conative scan ./my-project                 # Scan directory
  conative scan . --format json              # JSON output
  conative check --file src/main.ts          # Check single file
  conative check --content "import foo"      # Check inline content
  conative policy                            # Show current policy
  conative policy --format json > policy.json

EXIT CODES
  0  All checks passed (Compliant)
  1  Hard violation detected (blocked)
  2  Soft concern detected (warning)
  3  Error during execution

MORE INFO
  https://github.com/hyperpolymath/conative-gating
"#)]
#[command(after_help = r#"
REVERSIBILITY
  All conative commands are non-destructive read operations.
  No --undo or --revert is needed as no changes are made.

DRY RUN
  Use --dry-run on any command to see what would be checked
  without actually performing the full analysis.

SHELL COMPLETIONS
  Generate completions with:
    conative completions bash > /etc/bash_completion.d/conative
    conative completions zsh > ~/.zfunc/_conative
    conative completions fish > ~/.config/fish/completions/conative.fish
"#)]
struct Cli {
    /// Output verbosity level
    #[arg(short, long, value_enum, default_value = "normal", global = true)]
    verbosity: Verbosity,

    /// Dry run mode - show what would be done without doing it
    #[arg(long, global = true)]
    dry_run: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Custom policy file (Nickel .ncl or JSON)
    #[arg(short, long, global = true)]
    policy_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory tree for policy violations
    ///
    /// Recursively scans all files in the specified directory,
    /// checking each against the configured policy rules.
    ///
    /// Skips hidden directories, node_modules, target/, and _build/
    /// by default. Use --include-hidden to scan hidden files.
    #[command(visible_alias = "s")]
    Scan {
        /// Path to scan (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Include hidden files and directories
        #[arg(long)]
        include_hidden: bool,

        /// Maximum directory depth to scan (0 = unlimited)
        #[arg(short, long, default_value = "0")]
        depth: usize,

        /// File patterns to include (glob syntax)
        #[arg(short = 'I', long)]
        include: Vec<String>,

        /// File patterns to exclude (glob syntax)
        #[arg(short = 'E', long)]
        exclude: Vec<String>,
    },

    /// Check a single file or inline content
    ///
    /// Validates the provided content against policy rules.
    /// Either --file or --content must be specified.
    ///
    /// EXAMPLES
    ///   conative check --file src/utils.ts
    ///   conative check --content "const x: string = 'hello'"
    ///   cat file.py | conative check --content -
    #[command(visible_alias = "c")]
    Check {
        /// File path to check
        #[arg(short = 'F', long, group = "input")]
        file: Option<PathBuf>,

        /// Content string to check (use '-' for stdin)
        #[arg(short = 'C', long, group = "input")]
        content: Option<String>,

        /// Assumed file path for content (affects language detection)
        #[arg(short = 'a', long)]
        assume_path: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Display the current policy configuration
    ///
    /// Shows all language tiers, toolchain rules, forbidden patterns,
    /// and exceptions. Use --format json to export for modification.
    ///
    /// POLICY TIERS
    ///   Tier 1: Preferred languages (Rust, Elixir, Zig, Ada, Haskell, ReScript)
    ///   Tier 2: Acceptable languages (Nickel, Racket) - generates warnings
    ///   Forbidden: Blocked languages (TypeScript, Python, Go, Java)
    #[command(visible_alias = "p")]
    Policy {
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Show only specific section (languages, toolchain, patterns)
        #[arg(short, long)]
        section: Option<String>,
    },

    /// Validate a proposal JSON file
    ///
    /// Checks a structured proposal against all policy rules.
    /// Used for programmatic integration with LLM systems.
    ///
    /// PROPOSAL FORMAT
    ///   {
    ///     "id": "uuid",
    ///     "action_type": {"CreateFile": {"path": "..."}},
    ///     "content": "file contents",
    ///     "files_affected": ["path1", "path2"],
    ///     "llm_confidence": 0.95
    ///   }
    #[command(visible_alias = "v")]
    Validate {
        /// Proposal JSON file (use '-' for stdin)
        proposal: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Return non-zero exit code on any concern (not just violations)
        #[arg(long)]
        strict: bool,
    },

    /// Initialize policy configuration in current directory
    ///
    /// Creates a .conative/ directory with default configuration files
    /// that can be customized for project-specific policies.
    ///
    /// FILES CREATED
    ///   .conative/policy.ncl   - Main policy configuration
    ///   .conative/local.ncl    - Local overrides (gitignored)
    ///
    /// REVERSIBILITY
    ///   Remove with: rm -rf .conative/
    #[command(visible_alias = "i")]
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,

        /// Create minimal configuration
        #[arg(long)]
        minimal: bool,
    },

    /// Generate shell completions
    ///
    /// Outputs shell completion scripts to stdout.
    /// Redirect to appropriate location for your shell.
    ///
    /// EXAMPLES
    ///   conative completions bash > ~/.local/share/bash-completion/completions/conative
    ///   conative completions zsh > ~/.zfunc/_conative
    ///   conative completions fish > ~/.config/fish/completions/conative.fish
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Generate man page
    ///
    /// Outputs a man page in roff format.
    ///
    /// EXAMPLE
    ///   conative man > /usr/local/share/man/man1/conative.1
    Man,

    /// Run the gating contract test runner
    ///
    /// Executes contract tests from JSON files or evaluates requests
    /// using the formal gating contract specification.
    ///
    /// The contract defines:
    /// - Inputs (GatingRequest): What the gating system receives
    /// - Outputs (GatingDecision): What it returns
    /// - Refusal Taxonomy: Categorization of all refusals
    /// - Audit Log Format: Structured logging for compliance
    ///
    /// EXAMPLES
    ///   conative contract test training/           # Run all tests in directory
    ///   conative contract eval request.json        # Evaluate single request
    ///   conative contract eval request.json --audit  # With audit log output
    #[command(visible_alias = "ct")]
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },
}

#[derive(Subcommand)]
enum ContractAction {
    /// Run contract tests from test case files
    ///
    /// Reads JSON test case files and validates contract behavior.
    /// Returns non-zero exit code if any tests fail.
    Test {
        /// Directory or file containing test cases
        #[arg(default_value = "training")]
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,
    },

    /// Evaluate a gating request through the contract
    ///
    /// Processes a GatingRequest JSON and returns a GatingDecision.
    Eval {
        /// Request JSON file (use '-' for stdin)
        request: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Include audit log entry in output
        #[arg(long)]
        audit: bool,
    },

    /// Display contract schema information
    ///
    /// Shows the contract version, input/output schemas, and refusal codes.
    Schema {
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Show only specific section (inputs, outputs, refusals, audit)
        #[arg(short, long)]
        section: Option<String>,
    },

    /// Run red-team adversarial tests
    ///
    /// Executes adversarial test cases designed to bypass the gating system.
    /// Reports on bypass rates, false positives, and security score.
    ///
    /// CATEGORIES
    ///   bypass:      Attempts to bypass via docs/comments
    ///   obfuscation: Marker splitting, case variation
    ///   encoding:    Base64/hex encoded secrets
    ///   boundary:    Empty files, unicode, edge cases
    ///   injection:   Polyglot files, hidden secrets
    #[command(visible_alias = "rt")]
    Redteam {
        /// Directory containing red-team test cases
        #[arg(default_value = "training/redteam")]
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Show details of bypasses
        #[arg(long)]
        verbose: bool,
    },

    /// Regression testing against baseline
    ///
    /// Compare current test results against a saved baseline to detect
    /// regressions (tests that used to pass but now fail) and improvements.
    ///
    /// WORKFLOW
    ///   1. Run tests and save baseline: conative contract regression --save
    ///   2. Make changes to codebase
    ///   3. Compare against baseline: conative contract regression
    #[command(visible_alias = "reg")]
    Regression {
        /// Directory containing test cases
        #[arg(default_value = "training")]
        path: PathBuf,

        /// Baseline file path
        #[arg(short, long, default_value = ".conative/baseline.json")]
        baseline: PathBuf,

        /// Save current results as new baseline
        #[arg(long)]
        save: bool,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Fail on any regression
        #[arg(long)]
        strict: bool,
    },
}

fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let oracle = Oracle::with_rsr_defaults();

    let exit_code = match cli.command {
        Commands::Scan {
            path,
            format,
            include_hidden: _,
            depth: _,
            include: _,
            exclude: _,
        } => {
            if cli.dry_run {
                println!("[dry-run] Would scan: {}", path.display());
                println!("[dry-run] Format: {:?}", format);
                0
            } else {
                scan_directory(&oracle, &path, &format, &cli.verbosity)
            }
        }
        Commands::Check {
            file,
            content,
            assume_path,
            format,
        } => {
            if cli.dry_run {
                println!("[dry-run] Would check: {:?} or content", file);
                0
            } else {
                check_content(&oracle, file, content, assume_path, &format, &cli.verbosity)
            }
        }
        Commands::Policy { format, section } => {
            show_policy(&format, section.as_deref());
            0
        }
        Commands::Validate {
            proposal,
            format,
            strict,
        } => {
            if cli.dry_run {
                println!("[dry-run] Would validate: {}", proposal.display());
                0
            } else {
                validate_proposal(&oracle, &proposal, &format, strict)
            }
        }
        Commands::Init { force, minimal } => {
            if cli.dry_run {
                println!("[dry-run] Would create .conative/ directory");
                println!("[dry-run] Force: {}, Minimal: {}", force, minimal);
                0
            } else {
                init_config(force, minimal)
            }
        }
        Commands::Completions { shell } => {
            generate_completions(shell);
            0
        }
        Commands::Man => {
            generate_man_page();
            0
        }
        Commands::Contract { action } => match action {
            ContractAction::Test {
                path,
                format,
                fail_fast,
            } => {
                if cli.dry_run {
                    println!("[dry-run] Would run contract tests from: {}", path.display());
                    0
                } else {
                    run_contract_tests(&path, &format, fail_fast, &cli.verbosity)
                }
            }
            ContractAction::Eval {
                request,
                format,
                audit,
            } => {
                if cli.dry_run {
                    println!("[dry-run] Would evaluate request: {}", request.display());
                    0
                } else {
                    eval_contract_request(&request, &format, audit)
                }
            }
            ContractAction::Schema { format, section } => {
                show_contract_schema(&format, section.as_deref());
                0
            }
            ContractAction::Redteam {
                path,
                format,
                verbose,
            } => {
                if cli.dry_run {
                    println!("[dry-run] Would run red-team tests from: {}", path.display());
                    0
                } else {
                    run_redteam_tests(&path, &format, verbose, &cli.verbosity)
                }
            }
            ContractAction::Regression {
                path,
                baseline,
                save,
                format,
                strict,
            } => {
                if cli.dry_run {
                    println!("[dry-run] Would run regression tests");
                    println!("[dry-run] Tests: {}, Baseline: {}", path.display(), baseline.display());
                    0
                } else {
                    run_regression_tests(&path, &baseline, save, &format, strict, &cli.verbosity)
                }
            }
        },
    };

    std::process::exit(exit_code);
}

fn scan_directory(oracle: &Oracle, path: &Path, format: &OutputFormat, verbosity: &Verbosity) -> i32 {
    if matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
        eprintln!("Scanning: {}", path.display());
    }

    match oracle.scan_directory(path) {
        Ok(result) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                OutputFormat::Compact => {
                    let status = if !result.violations.is_empty() {
                        "VIOLATION"
                    } else if !result.concerns.is_empty() {
                        "CONCERN"
                    } else {
                        "OK"
                    };
                    println!(
                        "{} {} files={} violations={} concerns={}",
                        status,
                        result.path.display(),
                        result.files_scanned,
                        result.violations.len(),
                        result.concerns.len()
                    );
                }
                OutputFormat::Text => {
                    print_scan_result(&result);
                }
            }

            if !result.violations.is_empty() {
                1 // Hard violation
            } else if !result.concerns.is_empty() {
                2 // Soft concern
            } else {
                0 // Compliant
            }
        }
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            3
        }
    }
}

fn print_scan_result(result: &DirectoryScanResult) {
    println!("=== Conative Gating Scan Results ===\n");
    println!("Path: {}", result.path.display());
    println!("Files scanned: {}", result.files_scanned);
    println!("Verdict: {:?}\n", result.verdict);

    if !result.violations.is_empty() {
        println!("VIOLATIONS ({}):", result.violations.len());
        for v in &result.violations {
            println!("  {} - {:?}", v.file.display(), v.violation);
        }
        println!();
    }

    if !result.concerns.is_empty() {
        println!("CONCERNS ({}):", result.concerns.len());
        for c in &result.concerns {
            println!("  {} - {:?}", c.file.display(), c.concern);
        }
        println!();
    }

    if result.violations.is_empty() && result.concerns.is_empty() {
        println!("No violations or concerns found.");
    }
}

fn check_content(
    oracle: &Oracle,
    file: Option<PathBuf>,
    content: Option<String>,
    assume_path: Option<String>,
    format: &OutputFormat,
    verbosity: &Verbosity,
) -> i32 {
    let (content_str, file_path) = match (file, content) {
        (Some(f), _) => {
            if matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
                eprintln!("Reading file: {}", f.display());
            }
            let content = match std::fs::read_to_string(&f) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);
                    return 3;
                }
            };
            (content, f.to_string_lossy().to_string())
        }
        (None, Some(c)) => {
            let path = assume_path.unwrap_or_else(|| "stdin".to_string());
            (c, path)
        }
        (None, None) => {
            eprintln!("Either --file or --content must be provided");
            return 3;
        }
    };

    let proposal = Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: file_path.clone(),
        },
        content: content_str,
        files_affected: vec![file_path],
        llm_confidence: 1.0,
    };

    match oracle.check_proposal(&proposal) {
        Ok(result) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                OutputFormat::Compact => {
                    let status = if !result.violations.is_empty() {
                        "VIOLATION"
                    } else if !result.concerns.is_empty() {
                        "CONCERN"
                    } else {
                        "OK"
                    };
                    println!("{} violations={} concerns={}", status, result.violations.len(), result.concerns.len());
                }
                OutputFormat::Text => {
                    println!("=== Check Result ===\n");
                    println!("Verdict: {:?}\n", result.verdict);

                    if !result.violations.is_empty() {
                        println!("VIOLATIONS:");
                        for v in &result.violations {
                            println!("  [{}] {:?}", v.rule, v.violation_type);
                        }
                    }

                    if !result.concerns.is_empty() {
                        println!("CONCERNS:");
                        for c in &result.concerns {
                            println!(
                                "  [{}] {} - {}",
                                c.rule,
                                c.suggestion,
                                c.concern_type.clone().into_string()
                            );
                        }
                    }

                    if result.violations.is_empty() && result.concerns.is_empty() {
                        println!("Content is compliant.");
                    }
                }
            }

            if !result.violations.is_empty() {
                1
            } else if !result.concerns.is_empty() {
                2
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("Error checking content: {}", e);
            3
        }
    }
}

fn show_policy(format: &OutputFormat, section: Option<&str>) {
    let policy = Policy::rsr_default();

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&policy).unwrap());
        }
        OutputFormat::Compact => {
            println!(
                "policy tier1={} tier2={} forbidden={} exceptions={}",
                policy.languages.tier1.len(),
                policy.languages.tier2.len(),
                policy.languages.forbidden.len(),
                policy.languages.exceptions.len()
            );
        }
        OutputFormat::Text => {
            println!("=== RSR Default Policy ===\n");

            let show_all = section.is_none();
            let section = section.unwrap_or("");

            if show_all || section == "languages" {
                println!("TIER 1 (Preferred):");
                for lang in &policy.languages.tier1 {
                    println!("  + {} ({})", lang.name, lang.extensions.join(", "));
                }
                println!("\nTIER 2 (Acceptable):");
                for lang in &policy.languages.tier2 {
                    println!("  ~ {} ({})", lang.name, lang.extensions.join(", "));
                }
                println!("\nFORBIDDEN:");
                for lang in &policy.languages.forbidden {
                    println!("  - {} ({})", lang.name, lang.extensions.join(", "));
                }
                println!("\nEXCEPTIONS:");
                for exc in &policy.languages.exceptions {
                    println!(
                        "  {} allowed in: {} ({})",
                        exc.language,
                        exc.allowed_paths.join(", "),
                        exc.reason
                    );
                }
            }

            if show_all || section == "toolchain" {
                println!("\nTOOLCHAIN RULES:");
                for rule in &policy.toolchain.rules {
                    println!("  {} requires {}", rule.tool, rule.requires);
                }
            }

            if show_all || section == "patterns" {
                println!("\nFORBIDDEN PATTERNS:");
                for pattern in &policy.patterns.forbidden_patterns {
                    println!("  {} - {}", pattern.name, pattern.reason);
                }
            }
        }
    }
}

fn validate_proposal(oracle: &Oracle, proposal_path: &Path, format: &OutputFormat, strict: bool) -> i32 {
    let content = match std::fs::read_to_string(proposal_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read proposal file: {}", e);
            return 3;
        }
    };

    let proposal: Proposal = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse proposal JSON: {}", e);
            return 3;
        }
    };

    match oracle.check_proposal(&proposal) {
        Ok(result) => {
            match format {
                OutputFormat::Json | OutputFormat::Compact => {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                OutputFormat::Text => {
                    println!("Proposal: {}", result.proposal_id);
                    println!("Verdict: {:?}", result.verdict);
                    println!("Rules checked: {}", result.rules_checked.len());
                    println!("Violations: {}", result.violations.len());
                    println!("Concerns: {}", result.concerns.len());
                }
            }

            if !result.violations.is_empty() {
                1
            } else if strict && !result.concerns.is_empty() {
                2
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("Error validating proposal: {}", e);
            3
        }
    }
}

fn init_config(force: bool, minimal: bool) -> i32 {
    let config_dir = PathBuf::from(".conative");

    if config_dir.exists() && !force {
        eprintln!("Configuration directory already exists. Use --force to overwrite.");
        return 1;
    }

    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        eprintln!("Failed to create .conative directory: {}", e);
        return 3;
    }

    let policy_content = if minimal {
        r#"# Minimal Conative Policy
# Extend the RSR default with project-specific rules

let base = import "schema.ncl" in
{
  name = "Project Policy",
  extends = "rsr-default",
}
"#
    } else {
        include_str!("../config/policy.ncl")
    };

    let policy_path = config_dir.join("policy.ncl");
    if let Err(e) = std::fs::write(&policy_path, policy_content) {
        eprintln!("Failed to write policy.ncl: {}", e);
        return 3;
    }

    // Create local.ncl (gitignored)
    let local_content = r#"# Local policy overrides (not committed to git)
# Use this for machine-specific or developer-specific settings

{
  # local_exceptions = [],
}
"#;
    let local_path = config_dir.join("local.ncl");
    if let Err(e) = std::fs::write(&local_path, local_content) {
        eprintln!("Failed to write local.ncl: {}", e);
        return 3;
    }

    println!("Initialized Conative configuration in .conative/");
    println!("  .conative/policy.ncl  - Main policy configuration");
    println!("  .conative/local.ncl   - Local overrides (add to .gitignore)");
    println!();
    println!("To revert: rm -rf .conative/");

    0
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    clap_complete::generate(
        shell,
        &mut Cli::command(),
        "conative",
        &mut std::io::stdout(),
    );
}

fn generate_man_page() {
    use clap::CommandFactory;
    let man = clap_mangen::Man::new(Cli::command());
    let mut buffer: Vec<u8> = Vec::new();
    if let Err(e) = man.render(&mut buffer) {
        eprintln!("Failed to generate man page: {}", e);
        std::process::exit(3);
    }
    print!("{}", String::from_utf8_lossy(&buffer));
}

// Helper trait for ConcernType
trait IntoString {
    fn into_string(self) -> String;
}

impl IntoString for policy_oracle::ConcernType {
    fn into_string(self) -> String {
        match self {
            policy_oracle::ConcernType::VerbositySmell => "Verbosity smell".to_string(),
            policy_oracle::ConcernType::PatternDeviation => "Pattern deviation".to_string(),
            policy_oracle::ConcernType::UnusualStructure => "Unusual structure".to_string(),
            policy_oracle::ConcernType::Tier2Language { language } => {
                format!("Tier 2 language: {}", language)
            }
        }
    }
}

// ============ Contract Runner Functions ============

fn run_contract_tests(path: &Path, format: &OutputFormat, fail_fast: bool, verbosity: &Verbosity) -> i32 {
    let mut harness = TestHarness::new();
    let test_cases = match load_test_cases(path, verbosity) {
        Ok(cases) => cases,
        Err(e) => {
            eprintln!("Error loading test cases: {}", e);
            return 3;
        }
    };

    if test_cases.is_empty() {
        eprintln!("No test cases found in: {}", path.display());
        return 3;
    }

    if matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
        eprintln!("Running {} test cases...", test_cases.len());
    }

    for test in &test_cases {
        let result = harness.run_test(test);

        if matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
            let status = if result.passed { "PASS" } else { "FAIL" };
            eprintln!("  {} {} ({}μs)", status, test.name, result.duration_us);
        }

        if fail_fast && !result.passed {
            break;
        }
    }

    let summary = harness.summary();

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        OutputFormat::Compact => {
            println!(
                "tests={} passed={} failed={} duration={}μs",
                summary.total, summary.passed, summary.failed, summary.total_duration_us
            );
        }
        OutputFormat::Text => {
            println!("=== Contract Test Results ===\n");
            println!("Total:   {}", summary.total);
            println!("Passed:  {}", summary.passed);
            println!("Failed:  {}", summary.failed);
            println!("Duration: {}μs\n", summary.total_duration_us);

            if !summary.all_passed() {
                println!("Failed tests:");
                for name in summary.failed_tests() {
                    println!("  - {}", name);
                }

                // Show details of failures
                for result in &summary.results {
                    if !result.passed {
                        println!("\n  {} ERROR:", result.name);
                        if let Some(err) = &result.error {
                            println!("    {}", err);
                        }
                    }
                }
            } else {
                println!("All tests passed!");
            }
        }
    }

    if summary.all_passed() {
        0
    } else {
        1
    }
}

/// Load test cases from a file or directory
fn load_test_cases(path: &Path, verbosity: &Verbosity) -> Result<Vec<TestCase>, String> {
    let mut cases = Vec::new();

    if path.is_file() {
        cases.push(load_test_case_file(path)?);
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                // Recurse into subdirectories
                cases.extend(load_test_cases(&entry_path, verbosity)?);
            } else if entry_path.extension().map(|s| s == "json").unwrap_or(false) {
                match load_test_case_file(&entry_path) {
                    Ok(case) => cases.push(case),
                    Err(e) => {
                        if matches!(verbosity, Verbosity::Debug) {
                            eprintln!("Skipping {}: {}", entry_path.display(), e);
                        }
                    }
                }
            }
        }
    } else {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    Ok(cases)
}

/// Load a single test case from a training data JSON file
fn load_test_case_file(path: &Path) -> Result<TestCase, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    // Parse the training data format
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct TrainingData {
        proposal: Proposal,
        expected_verdict: String,
        #[serde(default)]
        reasoning: String,
        #[serde(default)]
        category: String,
        #[serde(default)]
        violation_type: Option<String>,
        #[serde(default)]
        concern_type: Option<String>,
        #[serde(default)]
        spirit_violation: bool,
    }

    let data: TrainingData = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let expected_verdict = match data.expected_verdict.as_str() {
        "Compliant" => Verdict::Allow,
        "HardViolation" => Verdict::Block,
        "SoftConcern" => Verdict::Warn,
        other => return Err(format!("Unknown verdict: {}", other)),
    };

    // Map the expected category based on violation_type, concern_type, or category
    let expected_category = if data.spirit_violation {
        // Spirit violations require SLM - these will fail until SLM is implemented
        Some(gating_contract::RefusalCategory::VerbositySmell)
    } else if let Some(ref vtype) = data.violation_type {
        match vtype.as_str() {
            "ForbiddenLanguage" => Some(gating_contract::RefusalCategory::ForbiddenLanguage),
            "ForbiddenToolchain" => Some(gating_contract::RefusalCategory::ForbiddenToolchain),
            "SecurityViolation" => Some(gating_contract::RefusalCategory::SecurityViolation),
            "ForbiddenPattern" => Some(gating_contract::RefusalCategory::ForbiddenPattern),
            _ => None,
        }
    } else if let Some(ref ctype) = data.concern_type {
        match ctype.as_str() {
            "VerbositySmell" => Some(gating_contract::RefusalCategory::VerbositySmell),
            "PatternDeviation" | "UnusualStructure" => {
                Some(gating_contract::RefusalCategory::StructuralAnomaly)
            }
            _ => None,
        }
    } else {
        match data.category.as_str() {
            "language" => {
                if data.expected_verdict == "HardViolation" {
                    Some(gating_contract::RefusalCategory::ForbiddenLanguage)
                } else {
                    None
                }
            }
            "toolchain" => Some(gating_contract::RefusalCategory::ForbiddenToolchain),
            "pattern" | "security" => Some(gating_contract::RefusalCategory::ForbiddenPattern),
            "spirit" => Some(gating_contract::RefusalCategory::VerbositySmell),
            _ => None,
        }
    };

    Ok(TestCase {
        name: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
        description: data.reasoning,
        request: GatingRequest::new(data.proposal),
        expected_verdict,
        expected_category,
        expected_code: None,
    })
}

fn eval_contract_request(request_path: &Path, format: &OutputFormat, include_audit: bool) -> i32 {
    let content = match std::fs::read_to_string(request_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read request file: {}", e);
            return 3;
        }
    };

    let request: GatingRequest = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to parse request JSON: {}", e);
            return 3;
        }
    };

    let runner = ContractRunner::new();
    let decision = match runner.evaluate(&request) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error evaluating request: {}", e);
            return 3;
        }
    };

    match format {
        OutputFormat::Json => {
            if include_audit {
                let audit = runner.audit(&request, &decision);
                #[derive(serde::Serialize)]
                struct Output {
                    decision: gating_contract::GatingDecision,
                    audit: AuditEntry,
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&Output {
                        decision: decision.clone(),
                        audit
                    })
                    .unwrap()
                );
            } else {
                println!("{}", serde_json::to_string_pretty(&decision).unwrap());
            }
        }
        OutputFormat::Compact => {
            let refusal_code = decision
                .refusal
                .as_ref()
                .map(|r| r.code.numeric())
                .unwrap_or(0);
            println!(
                "verdict={:?} code={} duration={}μs",
                decision.verdict, refusal_code, decision.processing.duration_us
            );
        }
        OutputFormat::Text => {
            println!("=== Gating Decision ===\n");
            println!("Request ID:  {}", decision.request_id);
            println!("Decision ID: {}", decision.decision_id);
            println!("Verdict:     {:?}", decision.verdict);
            println!("Duration:    {}μs", decision.processing.duration_us);

            if let Some(ref refusal) = decision.refusal {
                println!("\nRefusal Details:");
                println!("  Category: {}", refusal.category.display_name());
                println!("  Code:     {}", refusal.code.numeric());
                println!("  Message:  {}", refusal.message);
                if let Some(ref remediation) = refusal.remediation {
                    println!("  Fix:      {}", remediation);
                }
            }

            if include_audit {
                let audit = runner.audit(&request, &decision);
                println!("\nAudit Log Entry:");
                println!("{}", serde_json::to_string_pretty(&audit).unwrap());
            }
        }
    }

    decision.verdict.exit_code()
}

fn show_contract_schema(format: &OutputFormat, section: Option<&str>) {
    match format {
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct Schema {
                version: &'static str,
                schema: &'static str,
                inputs: InputSchema,
                outputs: OutputSchema,
                refusal_codes: Vec<RefusalCodeInfo>,
            }

            #[derive(serde::Serialize)]
            struct InputSchema {
                gating_request: Vec<&'static str>,
            }

            #[derive(serde::Serialize)]
            struct OutputSchema {
                gating_decision: Vec<&'static str>,
                verdicts: Vec<&'static str>,
            }

            #[derive(serde::Serialize)]
            struct RefusalCodeInfo {
                code: u16,
                name: &'static str,
                category: &'static str,
            }

            let schema = Schema {
                version: gating_contract::CONTRACT_VERSION,
                schema: gating_contract::CONTRACT_SCHEMA,
                inputs: InputSchema {
                    gating_request: vec![
                        "request_id: UUID",
                        "timestamp: DateTime<Utc>",
                        "proposal: Proposal",
                        "context: RequestContext",
                        "policy_override: Option<Policy>",
                    ],
                },
                outputs: OutputSchema {
                    gating_decision: vec![
                        "request_id: UUID",
                        "decision_id: UUID",
                        "timestamp: DateTime<Utc>",
                        "verdict: Verdict",
                        "refusal: Option<Refusal>",
                        "evaluations: EvaluationChain",
                        "processing: ProcessingMetadata",
                    ],
                    verdicts: vec!["Allow", "Warn", "Escalate", "Block"],
                },
                refusal_codes: vec![
                    RefusalCodeInfo { code: 100, name: "Lang100TypeScript", category: "ForbiddenLanguage" },
                    RefusalCodeInfo { code: 101, name: "Lang101Python", category: "ForbiddenLanguage" },
                    RefusalCodeInfo { code: 102, name: "Lang102Go", category: "ForbiddenLanguage" },
                    RefusalCodeInfo { code: 103, name: "Lang103Java", category: "ForbiddenLanguage" },
                    RefusalCodeInfo { code: 200, name: "Tool200NpmWithoutDeno", category: "ForbiddenToolchain" },
                    RefusalCodeInfo { code: 300, name: "Sec300HardcodedSecret", category: "SecurityViolation" },
                    RefusalCodeInfo { code: 500, name: "Spirit500Verbosity", category: "VerbositySmell" },
                ],
            };

            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
        OutputFormat::Compact | OutputFormat::Text => {
            let show_all = section.is_none();
            let section = section.unwrap_or("");

            println!("=== Gating Contract Schema ===\n");
            println!("Version: {}", gating_contract::CONTRACT_VERSION);
            println!("Schema:  {}", gating_contract::CONTRACT_SCHEMA);

            if show_all || section == "inputs" {
                println!("\n--- INPUTS ---\n");
                println!("GatingRequest:");
                println!("  request_id:      UUID (unique request identifier)");
                println!("  timestamp:       DateTime<Utc> (when request was created)");
                println!("  proposal:        Proposal (action_type, content, files_affected)");
                println!("  context:         RequestContext (source, session, repository)");
                println!("  policy_override: Option<Policy> (custom policy if needed)");
            }

            if show_all || section == "outputs" {
                println!("\n--- OUTPUTS ---\n");
                println!("GatingDecision:");
                println!("  request_id:  UUID (correlation with request)");
                println!("  decision_id: UUID (unique decision identifier)");
                println!("  timestamp:   DateTime<Utc> (when decision was made)");
                println!("  verdict:     Verdict (Allow | Warn | Escalate | Block)");
                println!("  refusal:     Option<Refusal> (details if not allowed)");
                println!("  evaluations: EvaluationChain (oracle, slm, arbiter results)");
                println!("  processing:  ProcessingMetadata (duration, rules checked)");
                println!("\nVerdicts:");
                println!("  Allow    (0) - Proposal proceeds");
                println!("  Warn     (2) - Proceed with warning");
                println!("  Escalate (3) - Requires human review");
                println!("  Block    (1) - Proposal rejected");
            }

            if show_all || section == "refusals" {
                println!("\n--- REFUSAL TAXONOMY ---\n");
                println!("Hard Policy Violations (Oracle):");
                println!("  100-199  ForbiddenLanguage   (TypeScript, Python, Go, Java...)");
                println!("  200-299  ForbiddenToolchain  (npm without deno, yarn...)");
                println!("  300-399  SecurityViolation   (hardcoded secrets, insecure hash...)");
                println!("  400-499  ForbiddenPattern    (forbidden imports, unsafe blocks...)");
                println!("\nSpirit Violations (SLM):");
                println!("  500-599  SpiritViolation     (verbosity, over-documentation...)");
                println!("\nSystem Codes:");
                println!("  900-999  SystemError         (invalid request, rate limited...)");
            }

            if show_all || section == "audit" {
                println!("\n--- AUDIT LOG FORMAT ---\n");
                println!("AuditEntry:");
                println!("  schema:           String (contract schema identifier)");
                println!("  audit_id:         UUID");
                println!("  request_id:       UUID");
                println!("  decision_id:      UUID");
                println!("  timestamp:        DateTime<Utc>");
                println!("  verdict:          Verdict");
                println!("  refusal_code:     Option<u16>");
                println!("  refusal_category: Option<RefusalCategory>");
                println!("  source:           String");
                println!("  repository:       Option<String>");
                println!("  session_id:       Option<String>");
                println!("  rules_checked:    Vec<String>");
                println!("  rules_triggered:  Vec<String>");
                println!("  duration_us:      u64");
                println!("  contract_version: String");
                println!("  content_hash:     String (SHA for verification)");
            }
        }
    }
}

// ============ Red-Team Test Functions ============

fn run_redteam_tests(path: &Path, format: &OutputFormat, verbose: bool, verbosity: &Verbosity) -> i32 {
    use std::collections::HashMap;

    let mut harness = TestHarness::new();
    let test_cases = match load_redteam_cases(path, verbosity) {
        Ok(cases) => cases,
        Err(e) => {
            eprintln!("Error loading red-team tests: {}", e);
            return 3;
        }
    };

    if test_cases.is_empty() {
        eprintln!("No red-team test cases found in: {}", path.display());
        return 3;
    }

    if matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
        eprintln!("Running {} red-team tests...", test_cases.len());
    }

    // Run all tests and collect results with category info
    let mut category_results: HashMap<String, (Vec<bool>, Vec<bool>, Vec<bool>)> = HashMap::new();
    let mut bypasses = Vec::new();
    let mut false_positives = Vec::new();

    for (test, redteam_category, attack_vector, is_fp_check) in &test_cases {
        let result = harness.run_test(test);

        let cat_key = format!("{:?}", redteam_category);
        let entry = category_results.entry(cat_key.clone()).or_insert((Vec::new(), Vec::new(), Vec::new()));

        if *is_fp_check {
            // False positive check: should pass (Allow)
            let is_fp = !result.passed && result.actual_verdict == Verdict::Block;
            entry.2.push(is_fp);
            if is_fp {
                false_positives.push((test.name.clone(), attack_vector.clone()));
            }
        } else {
            // Attack test: should block
            let was_blocked = result.actual_verdict == Verdict::Block;
            entry.0.push(was_blocked);
            if !was_blocked {
                bypasses.push((test.name.clone(), attack_vector.clone(), result.actual_verdict));
                entry.1.push(true);
            }
        }

        if verbose && matches!(verbosity, Verbosity::Verbose | Verbosity::Debug) {
            let status = if result.passed { "BLOCKED" } else { "BYPASS" };
            eprintln!("  {} [{}] {}", status, cat_key, test.name);
        }
    }

    // Build summary
    let mut by_category: HashMap<String, CategoryStats> = HashMap::new();
    let mut total_blocked = 0;
    let mut total_bypassed = 0;
    let mut total_fp = 0;

    for (cat, (blocked, bypassed, fps)) in &category_results {
        let blocked_count = blocked.iter().filter(|&&b| b).count();
        let bypassed_count = bypassed.len();
        let fp_count = fps.iter().filter(|&&f| f).count();

        total_blocked += blocked_count;
        total_bypassed += bypassed_count;
        total_fp += fp_count;

        by_category.insert(cat.clone(), CategoryStats {
            total: blocked.len() + bypassed.len() + fps.len(),
            blocked: blocked_count,
            bypassed: bypassed_count,
            false_positives: fp_count,
        });
    }

    let total = test_cases.len();
    let summary = RedTeamSummary {
        total,
        blocked: total_blocked,
        bypassed: total_bypassed,
        false_positives: total_fp,
        known_limitations: 0, // Could be parsed from test metadata
        by_category,
        bypass_rate: if total > 0 { total_bypassed as f64 / total as f64 } else { 0.0 },
        false_positive_rate: if total > 0 { total_fp as f64 / total as f64 } else { 0.0 },
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        OutputFormat::Compact => {
            println!(
                "redteam total={} blocked={} bypassed={} fps={} score={}",
                summary.total, summary.blocked, summary.bypassed, summary.false_positives, summary.security_score()
            );
        }
        OutputFormat::Text => {
            println!("=== Red-Team Test Results ===\n");
            println!("Total Tests:     {}", summary.total);
            println!("Blocked:         {} ({:.1}%)", summary.blocked, (summary.blocked as f64 / summary.total as f64) * 100.0);
            println!("Bypassed:        {} ({:.1}%)", summary.bypassed, summary.bypass_rate * 100.0);
            println!("False Positives: {} ({:.1}%)", summary.false_positives, summary.false_positive_rate * 100.0);
            println!("\nSecurity Score:  {}/100", summary.security_score());

            if !bypasses.is_empty() {
                println!("\n--- Bypasses ---");
                for (name, attack, verdict) in &bypasses {
                    println!("  {} [{:?}]", name, verdict);
                    if verbose {
                        println!("    Attack: {}", attack);
                    }
                }
            }

            if !false_positives.is_empty() {
                println!("\n--- False Positives ---");
                for (name, attack) in &false_positives {
                    println!("  {}", name);
                    if verbose {
                        println!("    Attack: {}", attack);
                    }
                }
            }

            println!("\n--- By Category ---");
            for (cat, stats) in &summary.by_category {
                println!("  {}: {} total, {} blocked, {} bypassed, {} fps",
                    cat, stats.total, stats.blocked, stats.bypassed, stats.false_positives);
            }
        }
    }

    if summary.has_unexpected_bypasses() {
        1
    } else {
        0
    }
}

/// Load red-team test cases with metadata
fn load_redteam_cases(path: &Path, verbosity: &Verbosity) -> Result<Vec<(TestCase, RedTeamCategory, String, bool)>, String> {
    let mut cases = Vec::new();

    if path.is_file() {
        if let Some(case) = load_redteam_file(path)? {
            cases.push(case);
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                cases.extend(load_redteam_cases(&entry_path, verbosity)?);
            } else if entry_path.extension().map(|s| s == "json").unwrap_or(false) {
                match load_redteam_file(&entry_path) {
                    Ok(Some(case)) => cases.push(case),
                    Ok(None) => {},
                    Err(e) => {
                        if matches!(verbosity, Verbosity::Debug) {
                            eprintln!("Skipping {}: {}", entry_path.display(), e);
                        }
                    }
                }
            }
        }
    } else {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    Ok(cases)
}

/// Load a single red-team test case
fn load_redteam_file(path: &Path) -> Result<Option<(TestCase, RedTeamCategory, String, bool)>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    #[derive(serde::Deserialize)]
    struct RedTeamData {
        proposal: Proposal,
        expected_verdict: String,
        #[serde(default)]
        reasoning: String,
        #[serde(default)]
        redteam_category: Option<String>,
        #[serde(default)]
        attack_vector: Option<String>,
    }

    let data: RedTeamData = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Skip non-redteam tests
    let redteam_cat = match &data.redteam_category {
        Some(c) => RedTeamCategory::from_str(c),
        None => return Ok(None),
    };

    let expected_verdict = match data.expected_verdict.as_str() {
        "Compliant" => Verdict::Allow,
        "HardViolation" => Verdict::Block,
        "SoftConcern" => Verdict::Warn,
        other => return Err(format!("Unknown verdict: {}", other)),
    };

    let is_fp_check = matches!(redteam_cat, RedTeamCategory::FalsePositiveCheck);

    let test_case = TestCase {
        name: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
        description: data.reasoning,
        request: GatingRequest::new(data.proposal),
        expected_verdict,
        expected_category: None,
        expected_code: None,
    };

    Ok(Some((test_case, redteam_cat, data.attack_vector.unwrap_or_default(), is_fp_check)))
}

// ============ Regression Test Functions ============

fn run_regression_tests(
    path: &Path,
    baseline_path: &Path,
    save_baseline: bool,
    format: &OutputFormat,
    strict: bool,
    verbosity: &Verbosity,
) -> i32 {
    // Run tests first
    let mut harness = TestHarness::new();
    let test_cases = match load_test_cases(path, verbosity) {
        Ok(cases) => cases,
        Err(e) => {
            eprintln!("Error loading test cases: {}", e);
            return 3;
        }
    };

    if test_cases.is_empty() {
        eprintln!("No test cases found in: {}", path.display());
        return 3;
    }

    for test in &test_cases {
        harness.run_test(test);
    }

    let summary = harness.summary();

    if save_baseline {
        // Create directory if needed
        if let Some(parent) = baseline_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("Failed to create baseline directory: {}", e);
                    return 3;
                }
            }
        }

        // Get git commit if available
        let git_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let baseline = RegressionBaseline::from_summary(&summary, git_commit);
        match baseline.to_json() {
            Ok(json) => {
                if let Err(e) = std::fs::write(baseline_path, &json) {
                    eprintln!("Failed to write baseline: {}", e);
                    return 3;
                }
                println!("Baseline saved to: {}", baseline_path.display());
                println!("Tests: {} total, {} passed, {} failed", summary.total, summary.passed, summary.failed);
                return 0;
            }
            Err(e) => {
                eprintln!("Failed to serialize baseline: {}", e);
                return 3;
            }
        }
    }

    // Compare against baseline
    let mut reg_harness = RegressionHarness::new();
    if baseline_path.exists() {
        if let Err(e) = reg_harness.load_baseline(baseline_path) {
            eprintln!("Failed to load baseline: {}", e);
            eprintln!("Run with --save to create a new baseline");
            return 3;
        }
    } else {
        eprintln!("No baseline found at: {}", baseline_path.display());
        eprintln!("Run with --save to create a new baseline");
        return 3;
    }

    reg_harness.add_results(summary.results.clone());
    let report = reg_harness.compare();

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        }
        OutputFormat::Compact => {
            println!(
                "regression compared={} stable={} regressed={} improved={} changed={} new={} removed={}",
                report.total_compared,
                report.stable_count,
                report.regressions.len(),
                report.improvements.len(),
                report.behavior_changes.len(),
                report.new_tests.len(),
                report.removed_tests.len()
            );
        }
        OutputFormat::Text => {
            println!("=== Regression Report ===\n");
            println!("{}", report.summary_text());

            if let Some(ref commit) = report.baseline_commit {
                println!("\nBaseline commit: {}", commit);
            }

            if !report.regressions.is_empty() {
                println!("\n--- REGRESSIONS ({}) ---", report.regressions.len());
                for reg in &report.regressions {
                    println!("  {} [{:?} -> {:?}]", reg.test_name, reg.baseline_verdict, reg.current_verdict);
                    if let Some(ref err) = reg.error_message {
                        println!("    Error: {}", err);
                    }
                }
            }

            if !report.improvements.is_empty() {
                println!("\n--- IMPROVEMENTS ({}) ---", report.improvements.len());
                for imp in &report.improvements {
                    println!("  {} [{:?} -> {:?}]", imp.test_name, imp.baseline_verdict, imp.current_verdict);
                }
            }

            if !report.behavior_changes.is_empty() {
                println!("\n--- BEHAVIOR CHANGES ({}) ---", report.behavior_changes.len());
                for change in &report.behavior_changes {
                    println!("  {} [{:?} -> {:?}]", change.test_name, change.baseline_verdict, change.current_verdict);
                }
            }

            if !report.new_tests.is_empty() {
                println!("\n--- NEW TESTS ({}) ---", report.new_tests.len());
                for name in &report.new_tests {
                    println!("  {}", name);
                }
            }

            if !report.removed_tests.is_empty() {
                println!("\n--- REMOVED TESTS ({}) ---", report.removed_tests.len());
                for name in &report.removed_tests {
                    println!("  {}", name);
                }
            }

            if report.has_regressions() {
                println!("\nWARNING: {} regression(s) detected!", report.regressions.len());
            } else if report.stable_count == report.total_compared {
                println!("\nAll tests stable.");
            }
        }
    }

    if strict && report.has_regressions() {
        1
    } else if report.has_regressions() {
        2 // Warning exit code
    } else {
        0
    }
}
