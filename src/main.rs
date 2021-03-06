#![deny(warnings)]

mod macros;
pub(crate) mod module_loader;
pub(crate) mod utils;

use crate::utils::{get_as_string, make_rt, next_id};
use backtrace::Backtrace;
use hirofa_utils::js_utils::adapters::JsRealmAdapter;
use hirofa_utils::js_utils::facades::JsRuntimeFacade;
use hirofa_utils::js_utils::Script;
use log::LevelFilter;
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use std::panic;
use std::sync::Arc;

async fn run(rt: Arc<QuickJsRuntimeFacade>, id: String) {
    let id2 = id.clone();
    match rt.js_loop_realm_sync(Some(&id), move |_q_js_rt, q_ctx| {
        let res = q_ctx.eval(Script::new(
            "test.js",
            &format!(
                r#"
                        async function main() {{
                            // uncomment the below lines to see the error
                            const {{ abc }} = await import('abc');
                            await abc({:?});
                            xconsole.log("{} running js...");
                            return 'test';
                        }}
                        main();
            "#,
                id2, id2
            ),
        ));

        // _q_js_rt.run_pending_jobs_if_any();

        match res {
            Ok(js) => q_ctx.to_js_value_facade(&js),
            Err(e) => Err(e),
        }
    }) {
        Ok(r) => {
            let fut = get_as_string(rt, r, "return value".to_owned(), id.clone()).await;
            match fut {
                Ok(val) => log!("{} result={}", id, val),
                Err(e) => eprintln!("err: {}", e),
            };
        }
        Err(e) => {
            eprintln!("error: {}", e);
        }
    };
}

#[allow(unused)]
async fn multi_context(rt: Arc<QuickJsRuntimeFacade>, seq: i32) -> anyhow::Result<()> {
    let mut list = vec![];
    for i in 0..10000 {
        let rt = rt.clone();
        let handle = tokio::task::spawn(async move {
            let id = format!("{}-{}", seq, next_id());
            let id2 = id.clone();
            log!("{}", id);
            // create new context for every execution
            match rt.create_context(&id) {
                Ok(_) => {}
                Err(e) => eprintln!("Error calling create_context {}: {}", id, e),
            };

            run(rt.clone(), id).await;
            // drop the above created context
            rt.drop_context(&id2);
        });
        list.push(handle);
        if i % 20 == 0 {
            for handle in list.iter_mut() {
                handle.await?;
            }
            list.clear();
        }
    }
    for handle in list.iter_mut() {
        handle.await?;
    }
    list.clear();
    Ok(())
}

#[allow(unused)]
async fn single_context(rt: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<()> {
    let id = next_id();
    let id2 = id.clone();
    // create new context for every execution
    match rt.create_context(&id) {
        Ok(_) => {}
        Err(e) => eprintln!("Error calling create_context {}: {}", id, e),
    };

    for i in 0..100000 {
        log!("{}", id);
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
        log!(
            "thread panic occurred: {}\nbacktrace: {:?}",
            panic_info,
            backtrace
        );
        log::error!(
            "thread panic occurred: {}\nbacktrace: {:?}",
            panic_info,
            backtrace
        );
    }));

    simple_logging::log_to_stderr(LevelFilter::Info);

    log!("start");

    let rt = make_rt();
    let mut list = vec![];
    for i in 0..10 {
        let rt = rt.clone();
        let handle = tokio::task::spawn_blocking(move || {
            tokio::task::spawn(async move {
                match multi_context(rt, i).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("{}", e),
                }
            })
        });
        list.push(handle);
    }
    for handle in list {
        let h = handle.await?;
        h.await?;
    }

    // single_context(rt).await?;
    // thread '<unnamed>' panicked at 'could not create func', .../quickjs_runtime-0.8.0/src/esvalue.rs:365:6
    // [after 7215]

    log!("done");

    Ok(())
}
