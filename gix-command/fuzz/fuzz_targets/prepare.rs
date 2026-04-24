#![no_main]

use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn inspect_prepare(command: &str) {
    for prep in [
        gix_command::prepare(command),
        gix_command::prepare(command).command_may_be_shell_script(),
        gix_command::prepare(command).command_may_be_shell_script_allow_manual_argument_splitting(),
        gix_command::prepare(command).command_may_be_shell_script_disallow_manual_argument_splitting(),
        gix_command::prepare(command).with_shell(),
        gix_command::prepare(command).with_shell().with_quoted_command(),
    ] {
        let command = std::process::Command::from(prep);
        _ = black_box(format!("{command:?}"));
    }
}

fuzz_target!(|command: &str| {
    inspect_prepare(command);
});
