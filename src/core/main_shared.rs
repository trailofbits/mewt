use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{CommandFactory, FromArgMatches};
use log::{debug, warn};

use crate::LanguageRegistry;
use crate::core::cli::{Args, Commands, PrintArgs};
use crate::core::cmds;
use crate::core::logging::init_logging;
use crate::core::store::SqlStore;
use crate::types::AppResult;
use crate::types::config::{CliOverrides, config, init_with_overrides, set_namespace};

pub async fn run_main(
    registry: Arc<LanguageRegistry>,
    namespace: &str,
    description: &str,
) -> AppResult<()> {
    // Set namespace at start (derives config/db filenames)
    set_namespace(namespace);

    // Override CLI help text with namespace and description
    // Leak strings to get 'static lifetime for clap
    let namespace_static: &'static str = Box::leak(namespace.to_string().into_boxed_str());
    let description_static: &'static str =
        Box::leak(format!("{} - {}", description, namespace).into_boxed_str());

    let mut cmd = Args::command();
    cmd = cmd.name(namespace_static).about(description_static);
    let matches = cmd.get_matches();
    let args = Args::from_arg_matches(&matches)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    // Handle global arguments
    if let Some(cwd_arg) = args.cwd.as_ref() {
        let cwd = PathBuf::from(cwd_arg).canonicalize()?;
        let _ = env::set_current_dir(&cwd);
    }
    let cwd = env::current_dir()?;
    debug!("Current working directory: {}", cwd.display());

    // Build CLI overrides for config precedence
    let cli_overrides = CliOverrides {
        db: args.db.clone(),
        log_level: args.log_level.clone(),
        log_color: args.log_color.clone(),
    };

    // Initialize configuration (files, env, then CLI overrides)
    init_with_overrides(&cli_overrides);

    // Initialize logging after config so level/color are applied
    init_logging();

    // Initialize the database
    let db_path = config().db();
    let db_file = PathBuf::from(&db_path);

    if !db_file.exists() {
        debug!(
            "Database file doesn't exist. Creating it at: {}",
            db_file.display()
        );
        let file = std::fs::File::create(&db_file)?;
        drop(file);
    }

    let db_connection_string = format!("sqlite:{db_path}");
    debug!("Using database: {db_connection_string}");
    let store = SqlStore::new(db_connection_string).await?;

    // Setup running flag to handle signals from ctrl-c
    let running = Arc::new(AtomicBool::new(true));
    let running_ctrlc = Arc::clone(&running);

    ctrlc::set_handler(move || {
        warn!("Received Ctrl-C, cleaning up..");
        running_ctrlc.store(false, Ordering::SeqCst);
    })
    .expect("Error creating a Ctrl-C handler");

    // Dispatch to appropriate command
    let exit_code = match args.command {
        Commands::Run(run_args) => {
            // Resolve command-specific options
            let resolved_targets = if !run_args.targets.is_empty()
                || run_args.ignore_targets.is_some()
            {
                Some(
                    config()
                        .resolve_targets(&run_args.targets, run_args.ignore_targets.as_deref())?,
                )
            } else {
                None
            };
            let mutations = config().resolve_mutations(run_args.mutations.as_deref());
            let test_cmd = config().resolve_test_cmd(run_args.test_cmd.as_deref());
            let test_timeout = config().resolve_test_timeout(run_args.test_timeout);

            let summary = cmds::execute_run(
                run_args,
                store,
                Arc::clone(&running),
                Arc::clone(&registry),
                resolved_targets,
                mutations,
                test_cmd,
                test_timeout,
            )
            .await?;

            // Determine exit code based on campaign results
            match summary {
                Some(_summary) if !running.load(Ordering::SeqCst) => {
                    // Campaign was interrupted
                    2
                }
                _ => {
                    // Successful completion (regardless of uncaught mutants)
                    0
                }
            }
        }
        Commands::Mutate(mutate_args) => {
            // Resolve command-specific options
            let resolved_targets = config()
                .resolve_targets(&mutate_args.targets, mutate_args.ignore_targets.as_deref())?;
            let mutations = config().resolve_mutations(None);

            cmds::execute_mutate(
                mutate_args,
                store,
                Arc::clone(&registry),
                resolved_targets,
                mutations,
            )
            .await?;
            0
        }
        Commands::Clean => {
            cmds::execute_clean(store).await?;
            0
        }
        Commands::Test(test_args) => {
            // Resolve command-specific options
            let test_cmd = config().resolve_test_cmd(test_args.test_cmd.as_deref());
            let test_timeout = config().resolve_test_timeout(test_args.test_timeout);

            cmds::execute_test(
                test_args,
                store,
                running,
                Arc::clone(&registry),
                test_cmd,
                test_timeout,
            )
            .await?;
            0
        }
        Commands::Purge(purge_args) => {
            cmds::execute_purge(purge_args, store).await?;
            0
        }
        Commands::Status(status_args) => {
            cmds::execute_status(status_args, store, Arc::clone(&registry)).await?;
            0
        }
        Commands::Results(args) => {
            cmds::execute_results(
                store,
                cmds::results::ResultsFilters {
                    target: args.target,
                    verbose: args.verbose,
                    id: args.id,
                    all: args.all,
                    status: args.status,
                    language: args.language,
                    mutation_type: args.mutation_type,
                    line: args.line,
                    file: args.file,
                    format: args.format,
                },
                &registry,
            )
            .await?;
            0
        }
        Commands::Print {
            command: print_args,
        } => {
            match print_args {
                PrintArgs::Mutations(args) => {
                    cmds::execute_print(
                        cmds::print::PrintCommand::Mutations(cmds::print::MutationsFilters {
                            language: args.language,
                            format: args.format,
                        }),
                        None,
                        Arc::clone(&registry),
                    )
                    .await?
                }
                PrintArgs::Targets(args) => {
                    cmds::execute_print(
                        cmds::print::PrintCommand::Targets(args.format),
                        Some(store),
                        Arc::clone(&registry),
                    )
                    .await?
                }
                PrintArgs::Mutant(args) => {
                    cmds::execute_print(
                        cmds::print::PrintCommand::Mutant(args.id),
                        Some(store),
                        Arc::clone(&registry),
                    )
                    .await?
                }
                PrintArgs::Mutants(args) => {
                    cmds::execute_print(
                        cmds::print::PrintCommand::Mutants(cmds::print::MutantsFilters {
                            target: args.target,
                            line: args.line,
                            file: args.file,
                            mutation_type: args.mutation_type,
                            tested: args.tested,
                            untested: args.untested,
                            format: args.format,
                        }),
                        Some(store),
                        Arc::clone(&registry),
                    )
                    .await?
                }
                PrintArgs::Config(args) => {
                    cmds::execute_print(
                        cmds::print::PrintCommand::Config(args.format),
                        None,
                        Arc::clone(&registry),
                    )
                    .await?
                }
            }
            0
        }
        Commands::Init => {
            cmds::execute_init().await?;
            0
        }
    };

    // Exit with appropriate code
    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
