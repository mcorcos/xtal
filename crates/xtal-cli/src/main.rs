//! `xtal` — binario de la CLI de Xtal (by UNIT).
//!
//! Parsea los argumentos con clap y despacha al handler correspondiente. Los errores
//! se imprimen de forma legible (con su cadena de contexto) y salen con código 1, en
//! vez de hacer panic: importante porque el cliente principal es Claude orquestando.

mod cli;
mod commands;
mod convert;
mod ctx;
mod deps;
mod example;
mod gen;
mod mcp;
mod setup;
mod update;
mod watch;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        // Imprime el error y toda su cadena de causas.
        eprintln!("Error: {err}");
        for cause in err.chain().skip(1) {
            eprintln!("  causa: {cause}");
        }
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let project = cli.project;
    let json = cli.json;
    match cli.command {
        Command::New(a) => commands::cmd_new(a, json),
        Command::Init(a) => commands::cmd_init(a, json),
        Command::Meas(cmd) => commands::cmd_meas(cmd, &project, json),
        Command::Plot(cmd) => commands::cmd_plot(cmd, &project, json),
        Command::Section(cmd) => commands::cmd_section(cmd, &project, json),
        Command::Circuit(cmd) => commands::cmd_circuit(cmd, &project, json),
        Command::Sim(cmd) => commands::cmd_sim(cmd, &project, json),
        Command::Raw(cmd) => commands::cmd_raw(cmd, &project, json),
        Command::Export(a) => commands::cmd_export(a, &project),
        Command::Run(a) => commands::cmd_run(a, &project),
        Command::Watch(a) => watch::cmd_watch(a, &project),
        Command::Config(cmd) => commands::cmd_config(cmd, &project),
        Command::Doctor(a) => commands::cmd_doctor(a, json),
        Command::Example(a) => example::cmd_example(a),
        Command::Update(a) => update::cmd_update(a),
        Command::Setup(a) => setup::cmd_setup(a),
        Command::Mcp(a) => mcp::cmd_mcp(a),
        Command::Completions(a) => gen::cmd_completions(a),
        Command::Man(a) => gen::cmd_man(a),
    }
}
