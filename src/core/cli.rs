use clap::{Parser, Subcommand};

/// mewt - Mutation testing framework
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// All relative paths will be interpreted relative to this directory.
    /// All child processes will be run in this directory.
    #[arg(long, global = true)]
    pub cwd: Option<String>,

    /// Location of the sqlite database
    #[arg(long, global = true)]
    pub db: Option<String>,

    /// Logging level (overrides env/config). One of: trace, debug, info, warn, error
    #[arg(long = "log.level", global = true)]
    pub log_level: Option<String>,

    /// Logging color control: "on" to force colors, "off" to disable; omit for auto
    #[arg(long = "log.color", global = true)]
    pub log_color: Option<String>,

    /// Comma-separated substrings; any target path containing any will be ignored
    #[arg(long = "ignore-targets", global = true)]
    pub ignore_targets: Option<String>,

    /// Comma-separated list of mutation slugs to test (e.g., "ER,CR").
    /// Run `mewt print mutations` for a list of slugs.
    /// If omitted, all mutation types are enabled.
    #[arg(long, global = true)]
    pub mutations: Option<String>,

    /// Test command for all targets (can be overridden per-command)
    #[arg(long = "test.cmd", global = true)]
    pub test_cmd: Option<String>,

    /// Test timeout in seconds (can be overridden per-command)
    #[arg(long = "test.timeout", global = true)]
    pub test_timeout: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new workspace (config + database)
    Init,
    /// Run a mutation testing campaign
    Run(RunArgs),

    /// Generate and save mutants for a target without running tests
    Mutate(MutateArgs),

    /// Clean the database of stale targets
    Clean,

    /// Show mutation testing results
    Results(ResultsArgs),

    /// Print various information about mutations and results
    Print {
        #[command(subcommand)]
        command: PrintArgs,
    },

    /// Show campaign overview with per-file breakdown and aggregates
    Status(StatusArgs),

    /// (Re-)Test a specific mutant by ID
    Test(TestArgs),

    /// Purge targets, mutants, and outcomes from the database
    Purge(PurgeArgs),
}

/// Arguments for the run command
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Target to mutate.
    /// If a file, mutate that file.
    /// If a directory, mutate all files inside the directory.
    /// If not provided, skip mutation generation and test existing mutants without outcomes.
    #[arg(value_name = "TARGET")]
    pub target: Option<String>,

    /// Test all mutants even if more severe mutants on the same line were uncaught.
    /// By default, less severe mutants are skipped if more severe ones were uncaught.
    #[arg(long)]
    pub comprehensive: bool,

    /// Stream stdout and stderr from baseline test to stdout
    #[arg(long)]
    pub verbose: bool,
}

/// Arguments for the mutate command
#[derive(Parser, Debug)]
pub struct MutateArgs {
    /// Target to mutate.
    /// If a file, mutate that file.
    /// If a directory, mutate all files inside the directory.
    #[arg(value_name = "TARGET")]
    pub target: String,
}

/// Arguments for the list-mutations command
#[derive(Parser, Debug)]
pub struct ListMutationsArgs {
    /// Target language for mutations
    #[arg(long)]
    pub language: Option<String>,
}

/// Arguments for the list-outcomes command
#[derive(Parser, Debug)]
pub struct ListOutcomesArgs {
    /// Filter outcomes by target path
    #[arg(long)]
    pub target: Option<String>,
}

/// Arguments for the print command
#[derive(Subcommand, Debug)]
pub enum PrintArgs {
    /// List all available mutations
    Mutations(PrintMutationsArgs),

    /// List all saved targets and their status
    Targets(PrintTargetsArgs),

    /// print a mutant file
    Mutant(PrintMutantArgs),

    /// List all mutants or filter by target
    Mutants(PrintMutantsArgs),
}

/// Arguments for the print targets subcommand
#[derive(Parser, Debug)]
pub struct PrintTargetsArgs {
    /// Output format: "table" (default) or "json"
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Arguments for the print mutations subcommand
#[derive(Parser, Debug)]
pub struct PrintMutationsArgs {
    /// Target language for mutations (omit to show all)
    #[arg(long)]
    pub language: Option<String>,

    /// Output format: "table" (default) or "json"
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Arguments for the results command
#[derive(Parser, Debug)]
pub struct ResultsArgs {
    /// Filter outcomes by target path
    #[arg(long)]
    pub target: Option<String>,

    /// Show verbose output including test output and timing information
    #[arg(long, default_value = "false")]
    pub verbose: bool,

    /// Show only the outcome for a specific mutant ID
    #[arg(long)]
    pub id: Option<i64>,

    /// Show all outcomes instead of only uncaught ones
    #[arg(long, default_value = "false")]
    pub all: bool,

    /// Filter by status (e.g., Uncaught, TestFail, Skipped, Timeout)
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by language (e.g., rust, python, javascript)
    #[arg(long)]
    pub language: Option<String>,

    /// Filter by mutation type slug (e.g., ER, CR, BR)
    #[arg(long)]
    pub mutation_type: Option<String>,

    /// Filter by line number
    #[arg(long)]
    pub line: Option<u32>,

    /// Filter by file path (substring match)
    #[arg(long)]
    pub file: Option<String>,

    /// Output format: "table" (default), "ids" (just IDs), "json", or "sarif"
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Arguments for the print mutants subcommand
#[derive(Parser, Debug)]
pub struct PrintMutantArgs {
    /// Print the target file mutated by this mutant ID
    #[arg(long)]
    pub id: i64,
}

/// Arguments for the print mutants subcommand
#[derive(Parser, Debug)]
pub struct PrintMutantsArgs {
    /// Filter mutants by target path
    #[arg(long)]
    pub target: Option<String>,

    /// Filter by line number
    #[arg(long)]
    pub line: Option<u32>,

    /// Filter by file path (substring match)
    #[arg(long)]
    pub file: Option<String>,

    /// Filter by mutation type slug (e.g., ER, CR, BR)
    #[arg(long)]
    pub mutation_type: Option<String>,

    /// Show only tested mutants (those with outcomes)
    #[arg(long)]
    pub tested: bool,

    /// Show only untested mutants (those without outcomes)
    #[arg(long)]
    pub untested: bool,

    /// Output format: "table" (default) or "ids" (just IDs, one per line)
    #[arg(long, default_value = "table")]
    pub format: String,
}

/// Arguments for the test command
#[derive(Parser, Debug)]
pub struct TestArgs {
    /// Comma-separated list of mutation IDs to test
    #[arg(long)]
    pub ids: Option<String>,

    /// Read mutant IDs from file (use '-' for stdin). Takes precedence over --ids.
    /// IDs should be separated by whitespace or newlines.
    #[arg(long)]
    pub ids_file: Option<String>,

    /// Stream stdout and stderr from baseline test to stdout
    #[arg(long)]
    pub verbose: bool,
}

/// Arguments for the purge command
#[derive(Parser, Debug)]
pub struct PurgeArgs {
    /// Target path to purge (if not provided, will purge all targets)
    #[arg(long)]
    pub target: Option<String>,
}

/// Arguments for the status command
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Output format: "table" (default) or "json"
    #[arg(long, default_value = "table")]
    pub format: String,
}
