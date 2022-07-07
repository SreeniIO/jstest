#![deny(warnings)]

pub(crate) mod module_loader;
mod utils;

use crate::utils::{get_as_string, js_debug, next_id};
use hirofa_utils::js_utils::{JsError, Script};
use module_loader::ModuleLoader;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::esvalue::EsValueFacade;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let collector = tracing_subscriber::registry()
        .with(EnvFilter::from_str("debug")?)
        .with(
            fmt::Subscriber::new()
                .with_thread_ids(true)
                .with_writer(non_blocking),
        );
    tracing::collect::set_global_default(collector).expect("Unable to set a global collector");

    let rt = Arc::new(
        QuickJsRuntimeBuilder::new()
            .memory_limit(1024 * 1024 * 6400)
            // .js_compiled_module_loader(IOCompiledModuleLoader::new(js_dir))
            .script_module_loader(Box::new(ModuleLoader::new()))
            // .script_pre_processor(TypeScriptPreProcessor::new())
            .build(),
    );

    rt.set_function(vec!["xconsole"], "log", js_debug)
        .ok()
        .expect("set_function failed");

    for i in 0..100000 {
        info!("{}", i);

        let id = next_id();
        let id2 = id.clone();
        match rt.create_context(&id) {
            Ok(_) => {}
            Err(e) => error!("Error calling create_context {}: {}", id, e),
        };

        match rt
            .add_rt_task_to_event_loop(move |q_js_rt| {
                // use the above created context run the eval
                // debug!("get_context: {}", ctx_id2);
                let q_ctx = match q_js_rt.opt_context(&id) {
                    Some(ctx) => ctx,
                    None => {
                        return Err(JsError::new_string(format!("Missing context {}!", &id)));
                    }
                };

                let res = q_ctx.eval(Script::new(
                    "test.js",
                    r#"
                    xconsole.log("running js...");
                    async function main() {
                        const { abc } = await import('test');
                        abc();
                    }
                    main();
                "#,
                ));

                match res {
                    Ok(js) => EsValueFacade::from_jsval(q_ctx, &js),
                    Err(e) => Err(e),
                }
            })
            .await
        {
            Ok(r) => {
                let fut = get_as_string(r, "return value".to_owned());
                match fut.await {
                    Ok(_) => {}
                    Err(e) => error!("{}", e),
                };
            }
            Err(e) => {
                error!("{}", e);
            }
        };

        rt.drop_context(&id2);
    }
    info!("done");

    Ok(())
}
