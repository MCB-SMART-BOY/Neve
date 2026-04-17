fn maybe_bind_final(name: Option<&str>, expr: &str) -> String {
    match name {
        Some(name) => format!("        let {name} = {expr};\n"),
        None => String::new(),
    }
}

pub fn shell_projection_source(final_binding: Option<&str>) -> String {
    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let final_binding = maybe_bind_final(final_binding, "same");

    format!(
        r#"
        import std.io as io;
        let migrated = io.execCommand(io.command("{program}", ["{flag}", "rustc --version"]));
        let canonical = io.execCommand(io.command("{program}", ["{flag}", "rustc --version"]));
        let same =
            typeOf(migrated) == "ProcessResult" &&
            io.processSuccess(migrated) == io.processSuccess(canonical) &&
            io.processStdout(migrated) == io.processStdout(canonical) &&
            io.processCode(migrated) == io.processCode(canonical) &&
            io.processStderr(migrated) == io.processStderr(canonical);
{final_binding}    "#
    )
}

pub fn pipeline_execution_source(final_binding: Option<&str>) -> String {
    let (
        producer_program,
        producer_flag,
        producer_cmd,
        consumer_program,
        consumer_flag,
        consumer_cmd,
    ) = if cfg!(windows) {
        ("cmd", "/C", "echo neve", "cmd", "/C", "findstr neve")
    } else {
        ("sh", "-c", "printf neve", "sh", "-c", "grep neve")
    };
    let final_binding = maybe_bind_final(final_binding, "same");

    format!(
        r#"
        import std.io as io;
        let result = io.execPipeline(io.pipeline([
            io.command("{producer_program}", ["{producer_flag}", "{producer_cmd}"]),
            io.command("{consumer_program}", ["{consumer_flag}", "{consumer_cmd}"])
        ]));
        let same =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "" &&
            io.processStderr(result) == "";
{final_binding}    "#
    )
}
