#![deny(warnings)]

use hirofa_utils::js_utils::modules::ScriptModuleLoader;
use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;

pub struct ModuleLoader;

impl ModuleLoader {
    pub fn new() -> Self {
        Self
    }
}

impl ScriptModuleLoader<QuickJsRealmAdapter> for ModuleLoader {
    fn normalize_path(
        &self,
        _ctx: &QuickJsRealmAdapter,
        _ref_path: &str,
        path: &str,
    ) -> Option<String> {
        Some(path.to_owned())
    }

    fn load_module(&self, _ctx: &QuickJsRealmAdapter, _absolute_path: &str) -> String {
        r#"
        export function abc() {
            xconsole.log("running module...");
        }
        "#
        .to_owned()
    }
}
