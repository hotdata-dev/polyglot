use crate::helpers::{
    err_result, map_polyglot_error, ok_json_result, ok_result, panic_result, required_arg,
};
use crate::types::{PolyglotResult, STATUS_INVALID_ARGUMENT, STATUS_SERIALIZATION_ERROR};
use polyglot_sql::builder::plan::{execute, BuildRequest, BuildResult};
use std::os::raw::c_char;

/// Evaluate a versioned, stateless SQL builder request.
#[no_mangle]
pub extern "C" fn polyglot_build(request_json: *const c_char) -> PolyglotResult {
    match std::panic::catch_unwind(|| build_impl(request_json)) {
        Ok(result) => result,
        Err(panic) => panic_result(panic),
    }
}

fn build_impl(request_json: *const c_char) -> PolyglotResult {
    let request_json = match unsafe { required_arg(request_json, "request_json") } {
        Ok(value) => value,
        Err(result) => return result,
    };
    let request: BuildRequest = match serde_json::from_str(&request_json) {
        Ok(request) => request,
        Err(error) => {
            return err_result(
                STATUS_SERIALIZATION_ERROR,
                format!("Invalid builder request JSON: {error}"),
            )
        }
    };
    match execute(&request) {
        Ok(BuildResult::Ast(expression)) => ok_json_result(&expression),
        Ok(BuildResult::Sql(sql)) => ok_result(sql),
        Err(error) => err_result(
            map_polyglot_error(&error, STATUS_INVALID_ARGUMENT),
            error.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn builds_sql_from_json() {
        let request = CString::new(
            r#"{
            "version": 1,
            "read_dialect": "generic",
            "plan": {
                "base": {"kind":"select","expressions":[{"kind":"sql","sql":"x"}]},
                "operations": []
            },
            "output": {"kind":"sql","dialect":"generic"}
        }"#,
        )
        .unwrap();
        let result = polyglot_build(request.as_ptr());
        assert_eq!(result.status, 0);
        let value = unsafe { CStr::from_ptr(result.data) }.to_str().unwrap();
        assert_eq!(value, "SELECT x");
        crate::memory::polyglot_free_result(result);
    }
}
