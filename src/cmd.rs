use crate::api::{metadata, ApiManager};
use crate::arg::CliInput;
use clap::{command, Arg, Command};

pub fn cmd() -> Command {
    cmd_base().subcommand(cmd_api_stub())
}

fn cmd_base() -> Command {
    command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
}

fn cmd_api_stub() -> Command {
    cmd_api_base().disable_help_flag(true).arg(
        Arg::new("args")
            .num_args(0..)
            .trailing_var_arg(true)
            .allow_hyphen_values(true)
            .hide(true),
    )
}

pub fn cmd_api_base() -> Command {
    Command::new("api").about("Directly invoke the Azure API primitives.")
}

pub fn cmd_api(api_manager: &ApiManager, input: &CliInput) -> Command {
    let pos_args = input.pos_args();

    // No positional argument specified, list the rps
    if pos_args.is_empty() {
        return cmd_base().subcommand(
            cmd_api_base().subcommands(api_manager.list_rps().iter().map(Command::new)),
        );
    }
    let rp = pos_args.first().unwrap();
    if let Ok(metadata) = api_manager.read_metadata(rp) {
        let mut args = pos_args.iter();
        let mut command_names = vec![];

        // Construct a fake command here to initiate the following while loop
        let mut c: Option<metadata::Command> = Some(metadata::Command {
            name: rp.to_string(),
            command_groups: Some(metadata.command_groups),
            ..metadata::Command::default()
        });
        let mut cg: Option<metadata::CommandGroup> = None;
        while let Some(arg) = args.next() {
            command_names.push(arg.to_string());
            if cg.is_some() {
                c = cg
                    .unwrap()
                    .commands
                    .iter()
                    .find(|c| c.name.as_str() == *arg)
                    .cloned();
                cg = None;
            } else if c.is_some() {
                cg = c
                    .unwrap()
                    .command_groups
                    .and_then(|cg| cg.iter().find(|c| c.name.as_str() == *arg).cloned());
                c = None;
            } else {
                break;
            }
        }

        let mut command_names_rev = command_names.iter().rev();
        let mut cmd = Command::new(command_names_rev.next().unwrap());
        if let Some(c) = c {
            cmd = cmd.subcommands(c.arg_groups.iter().map(|ag| Command::new(ag.name.clone())))
        } else if let Some(cg) = cg {
            cmd = cmd.subcommands(cg.commands.iter().map(|c| Command::new(c.name.clone())));
        }
        for name in command_names_rev {
            cmd = Command::new(name.clone()).subcommand(cmd);
        }
        cmd_base().subcommand(cmd_api_base().subcommand(cmd))
    } else {
        cmd_base().subcommand(cmd_api_base())
    }
}

#[test]
fn verify_cmd() {
    cmd().debug_assert();
}
