use nanoshell::{
    codec::Value,
    shell::{Context, ContextOptions},
};

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let context = Context::new(ContextOptions {
        app_namespace: "nanoshellDemo".into(),
        ..Default::default()
    });

    let context = context.unwrap();

    context
        .window_manager
        .borrow_mut()
        .create_window(Value::Null, None);

    context.run_loop.borrow().run();
}
