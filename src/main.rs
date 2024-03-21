use std::io::Write;

use anyhow::{Context, Result};

use minijinja::{context, Environment, Error, ErrorKind};
use minijinja_vault::make_vault_client;

use refinery::Migration;
use refinery_core::{find_migration_files, MigrationType, Runner};
use snowflake_api::{AuthArgs, SnowflakeApiBuilder};

use clap::Parser;

#[derive(Parser, Debug, Clone)]
struct DryRunArgs {
    /// Whether to run the migrations or just generate the files
    #[clap(short, long, default_value = "false")]
    dry_run: bool,

    /// The directory to output the generated migration files if dry_run is true
    #[clap(short, long, default_value = "./target/migrations")]
    output_dir: String,
}

#[derive(Parser, Debug)]
struct Cli {
    /// The path to the migration files
    #[clap(short, long)]
    path: String,

    /// Whether to enable the vault client template extension
    #[clap(short, long, default_value = "false")]
    enable_vault: bool,

    #[clap(flatten)]
    dry_run_args: DryRunArgs,
}

/// This is infintely better than a boolean:
enum Mode {
    DryRun(DryRunArgs),
    Run,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let args = Cli::parse();
    let env = configure_jinja_env(args.enable_vault);

    let migration_files_path = find_migration_files(args.path, MigrationType::Sql)?;
    let mut migrations: Vec<Migration> = Vec::new();

    let mode = if args.dry_run_args.dry_run {
        Mode::DryRun(args.dry_run_args)
    } else {
        Mode::Run
    };

    // generate migrations from the files
    handle_migrations(migration_files_path, env, &mut migrations, &mode)?;

    if let Mode::Run = mode {
        let auth_args = AuthArgs::from_env()?;
        let mut conn = SnowflakeApiBuilder::new(auth_args).build()?;
        // we can't use an outer runtime because minijinja is a sync library
        // and under the hood to support vault we are spawning our own tokio runtime
        // so here we generate one to run the migrations
        let runtime = tokio::runtime::Runtime::new()?;

        let runner = Runner::new(&migrations);
        match runtime.block_on(runner.run_async(&mut conn)) {
            Ok(_) => tracing::info!("Migrations ran successfully"),
            Err(e) => tracing::error!("Error running migrations: {}", e),
        }
    }

    Ok(())
}

fn handle_migrations(
    migration_files_path: impl Iterator<Item = std::path::PathBuf>,
    env: Environment<'_>,
    migrations: &mut Vec<Migration>,
    mode: &Mode,
) -> Result<(), anyhow::Error> {
    for path in migration_files_path {
        tracing::info!("Reading migration file: {}", path.display());

        // safe to call unwrap as find_migration_filenames returns canonical paths
        let filename = path
            .clone()
            .file_stem()
            .and_then(|file| file.to_os_string().into_string().ok())
            .unwrap();

        let source = std::fs::read_to_string(path.clone())?;
        let render_result = env.render_named_str(&filename, &source, context! {});

        match render_result {
            Ok(sql) => {
                tracing::trace!("Templated SQL: {sql}");
                match mode {
                    Mode::DryRun(ref dry_run_args) => {
                        if dry_run_args.dry_run {
                            let output_dir = &dry_run_args.output_dir;
                            let output_path = format!("{}/{}.sql", output_dir, filename);

                            if !std::path::Path::new(output_dir).exists() {
                                std::fs::create_dir_all(output_dir)?;
                            }

                            let mut file = std::fs::File::create(output_path)?;
                            file.write_all(sql.as_bytes())?;
                        }
                    }
                    Mode::Run => {
                        let migration =
                            Migration::unapplied(&filename, &sql).with_context(|| {
                                format!("could not read migration file name {}", path.display())
                            })?;

                        migrations.push(migration);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Migration {} failed to render: {}", filename, e);
            }
        }
    }
    Ok(())
}

fn get_env(name: &str) -> Result<String, Error> {
    std::env::var(name)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, "env var not found").with_source(e))
}

#[tracing::instrument]
fn configure_jinja_env(enable_vault: bool) -> Environment<'static> {
    let mut env = Environment::new();
    // disable default autoescaping since we are not generating html
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);

    if enable_vault {
        // if the log level is DEBUG or higher and we detect CI, panic
        // this is to prevent leaking secrets in logs
        if (tracing::log::max_level() >= tracing::log::LevelFilter::Debug) && ci_info::is_ci() {
            panic!("Vault client is enabled, but log level is too high. Please lower the log level to INFO or lower to use the vault client");
        }

        // TODO: optimize this so that vault client is
        // just a global variable that we initialize once
        env.add_function("make_vault_client", make_vault_client);
    }
    // similar to schemachange we add a funtcion that
    // exposes env vars to the template
    env.add_function("get_env", get_env);
    env
}
