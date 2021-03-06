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

    fn load_module(&self, _ctx: &QuickJsRealmAdapter, absolute_path: &str) -> String {
        match absolute_path {
            "xyz" => {
                r#"
export async function xyz(id) {
    xconsole.log(`${id} running module xyz...`);
}
            "#
            }
            _ => {
                r#"
import { xyz } from 'xyz';
export async function abc(id) {
    await xyz(id);
    xconsole.log(`${id} running module abc...`);
}
            "#
            }
        }
        .to_owned()
    }
}
