#![deny(warnings)]

use crate::module_loader::ModuleLoader;
use async_recursion::async_recursion;
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
pub async fn get_as_string(val: EsValueFacade, reason: String) -> anyhow::Result<String, JsError> {
    if val.is_string() {
        Ok(val.get_str().to_owned())
    } else if val.is_promise() {
        // println!("resolving promise");
        let fut = val.get_promise_result();
        let val = fut.await;
        match val {
            Ok(r) => {
                // println!("promise resolved: {:?}", r);
                get_as_string(r, reason).await
            }
            Err(e) => {
                if e.is_error() {
                    return Err(e.get_error());
                }
                if e.is_object() {
                    let obj = e.get_object().ok().unwrap();
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
                }
                match e.stringify() {
                    Ok(s) => Err(JsError::new_string(s)),
                    Err(e) => Err(e),
                }
            }
        }
    } else if val.is_object() {
        Ok(format!("{:?}", val.stringify().ok().unwrap()))
    } else if val.is_undefined() {
        Ok("".to_owned())
    } else {
        Err(JsError::new_string(format!(
            "Unexpected value found `{:?}` for {}, expected a string.",
            val, reason
        )))
    }
}

pub fn make_rt() -> Arc<QuickJsRuntimeFacade> {
    let rt = Arc::new(
        QuickJsRuntimeBuilder::new()
            .memory_limit(1024 * 1024 * 6400)
            .script_module_loader(Box::new(ModuleLoader::new()))
            .build(),
    );
    rt.set_function(vec!["xconsole"], "log", js_debug)
        .ok()
        .expect("set_function failed");
    rt
}
