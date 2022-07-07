#![deny(warnings)]

pub(crate) mod module_loader;
mod utils;

use crate::utils::{get_as_string, make_rt, next_id};
use hirofa_utils::js_utils::{JsError, Script};
use quickjs_runtime::esvalue::EsValueFacade;
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use std::sync::Arc;

async fn run(rt: Arc<QuickJsRuntimeFacade>, id: String) {
    match rt
        .add_rt_task_to_event_loop(move |q_js_rt| {
            // use the above created context for eval
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
                Err(e) => eprintln!("{}", e),
            };
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    };
}

#[allow(unused)]
async fn multi_context(rt: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<()> {
    for i in 0..100000 {
        println!("{}", i);

        let id = next_id();
        let id2 = id.clone();
        // create new context for every execution
        match rt.create_context(&id) {
            Ok(_) => {}
            Err(e) => eprintln!("Error calling create_context {}: {}", id, e),
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
        Err(e) => eprintln!("Error calling create_context {}: {}", id, e),
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
    let rt = make_rt();

    multi_context(rt).await?;
    // single_context(rt).await?;

    println!("done");

    Ok(())
}
