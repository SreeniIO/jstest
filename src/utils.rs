#![deny(warnings)]

use crate::module_loader::ModuleLoader;
use async_recursion::async_recursion;
use hirofa_utils::js_utils::facades::values::JsValueFacade;
use hirofa_utils::js_utils::facades::JsRuntimeFacade;
use hirofa_utils::js_utils::JsError;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::esvalue::{EsValueConvertible, EsValueFacade, ES_UNDEFINED};
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn next_id() -> String {
    let next_val = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("id-{}", next_val)
}

fn get_logger_msg(args: &[EsValueFacade]) -> Result<Option<String>, JsError> {
    match args.get(0) {
        Some(arg) => {
            if arg.is_string() {
                let val = arg.get_str();
                if val.len() > 2003 {
                    Ok(Some(format!("{}...", &val[..2000])))
                } else {
                    Ok(Some(arg.get_str().to_owned()))
                }
            } else if let Ok(msg) = arg.stringify() {
                if msg.len() > 2003 {
                    Ok(Some(format!("{}...", &msg[..2000])))
                } else {
                    Ok(Some(msg))
                }
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub fn js_debug(
    _: &QuickJsRealmAdapter,
    args: Vec<EsValueFacade>,
) -> Result<EsValueFacade, JsError> {
    if let Some(msg) = get_logger_msg(&args)? {
        println!("{}", msg);
    }
    Ok(ES_UNDEFINED.to_es_value_facade())
}

#[async_recursion]
pub async fn get_as_string(
    rt: Arc<QuickJsRuntimeFacade>,
    val: JsValueFacade,
    reason: String,
    id: String,
) -> anyhow::Result<String, JsError> {
    println!("{} get_as_string={}", id, val.get_value_type());
    match val {
        JsValueFacade::String { val } => Ok(val),
        JsValueFacade::JsObject { cached_object } => {
            let week_rti = rt.js_get_runtime_facade_inner();
            let rti = week_rti.upgrade().unwrap();
            let obj = cached_object.js_get_object(&*rti).await?;
            let stack = obj.get("stack");
            let title = obj.get("title");
            let message = obj.get("message");
            if let (Some(title), Some(message), Some(stack)) = (title, message, stack) {
                let title = if title.is_string() {
                    title.get_str()
                } else {
                    ""
                };
                let message = if message.is_string() {
                    message.get_str()
                } else {
                    "Unexpected JS Error! Please check the server log."
                };
                let stack = if stack.is_string() {
                    stack.get_str()
                } else {
                    ""
                };
                return Err(JsError::new(
                    title.to_owned(),
                    message.to_owned(),
                    stack.to_owned(),
                ));
            };
            todo!()
        }
        JsValueFacade::JsPromise { cached_promise } => {
            let week_rti = rt.js_get_runtime_facade_inner();
            let rti = week_rti.upgrade().unwrap();
            let val = cached_promise.js_get_promise_result(&*rti).await?;
            match val {
                Ok(val) => get_as_string(rt, val, reason, id).await,
                Err(e) => Ok(format!("{:?}", e)),
            }
        }
        JsValueFacade::Undefined | JsValueFacade::Null => Ok("".to_owned()),
        JsValueFacade::JsError { val } => Err(val),
        _ => Err(JsError::new_string(format!(
            "Unexpected value found `{:?}` for {}, expected a string.",
            val, reason
        ))),
    }
}

pub fn make_rt() -> Arc<QuickJsRuntimeFacade> {
    let rt = Arc::new(
        QuickJsRuntimeBuilder::new()
            // .memory_limit(1024 * 1024 * 6400)
            .script_module_loader(Box::new(ModuleLoader::new()))
            .build(),
    );
    rt.set_function(vec!["xconsole"], "log", js_debug)
        .ok()
        .expect("set_function failed");
    rt
}
