use crate::types::AppError;
use crate::utils::registry_cli::{
    is_not_found_text, output_text, run_command, run_command_with_timeout,
};
use std::time::Duration;

pub(crate) fn pnputil_command(args: &[&str]) -> Result<std::process::Output, AppError> {
    run_command("pnputil", args, &format!("pnputil {:?}", args))
}

pub(crate) fn pnputil_command_with_timeout(
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, AppError> {
    run_command_with_timeout("pnputil", args, &format!("pnputil {:?}", args), timeout)
}

pub(crate) fn pnputil_output_text(output: &std::process::Output) -> String {
    output_text(output)
}

pub(crate) fn pnputil_not_found_text(text: &str) -> bool {
    is_not_found_text(text)
}
