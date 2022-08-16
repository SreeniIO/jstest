#![deny(warnings)]

pub(crate) mod module_loader;
mod utils;

use crate::utils::{get_as_string, make_rt, next_id};
use backtrace::Backtrace;
use hirofa_utils::js_utils::adapters::JsRealmAdapter;
use hirofa_utils::js_utils::facades::JsRuntimeFacade;
use hirofa_utils::js_utils::Script;
use log::{debug, error, LevelFilter};
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use std::panic;
use std::sync::Arc;

async fn run(rt: Arc<QuickJsRuntimeFacade>, id: String) {
    match rt.js_loop_realm_sync(Some(&id), move |_q_js_rt, q_ctx| {
        let res = q_ctx.eval(Script::new(
            "test.js",
            r#"
                        async function main() {
                            // uncomment the below lines to see the error
                            const { abc } = await import('abc');
                            await abc();
                            xconsole.log("running js...");
                            return 'test';
                        }
                        main();
            "#,
        ));

        _q_js_rt.run_pending_jobs_if_any();

        match res {
            Ok(js) => q_ctx.to_js_value_facade(&js),
            Err(e) => Err(e),
        }
    }) {
        Ok(r) => {
            let fut = get_as_string(rt, r, "return value".to_owned(), id).await;
            match fut {
                Ok(val) => debug!("result={}", val),
                Err(e) => error!("err: {}", e),
            };
        }
        Err(e) => {
            error!("error: {}", e);
        }
    };
}

#[allow(unused)]
async fn multi_context(rt: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<()> {
    for i in 0..100000 {
        debug!("index={}", i);

        let id = next_id();
        let id2 = id.clone();
        // create new context for every execution
        match rt.create_context(&id) {
            Ok(_) => {}
            Err(e) => error!("Error calling create_context {}: {}", id, e),
        };

        run(rt.clone(), id).await;
        // drop the above created context
        rt.drop_context(&id2);

        // Segmentation fault: 11 [after 4520]
    }
    Ok(())
}

#[allow(unused)]
async fn single_context(rt: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<()> {
    let id = next_id();
    let id2 = id.clone();
    // create new context for every execution
    match rt.create_context(&id) {
        Ok(_) => {}
        Err(e) => error!("Error calling create_context {}: {}", id, e),
    };

    for i in 0..100000 {
        println!("{}", i);
        let id = id.clone();
        run(rt.clone(), id).await;

        // thread '<unnamed>' panicked at 'could not create func', .../quickjs_runtime-0.8.0/src/esvalue.rs:365:6
        // [after 7215]
    }

    // drop the above created context
    rt.drop_context(&id2);

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::new();
        println!(
            "thread panic occurred: {}\nbacktrace: {:?}",
            panic_info, backtrace
        );
        log::error!(
            "thread panic occurred: {}\nbacktrace: {:?}",
            panic_info,
            backtrace
        );
    }));

    simple_logging::log_to_file("jstest.log", LevelFilter::Trace).unwrap();

    let rt = make_rt();

    multi_context(rt).await?;
    // Segmentation fault: 11 [after 4520]

    // single_context(rt).await?;
    // thread '<unnamed>' panicked at 'could not create func', .../quickjs_runtime-0.8.0/src/esvalue.rs:365:6
    // [after 7215]

    debug!("done");

    Ok(())
}
