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
        #[arg(short, long, group = "input")]
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
