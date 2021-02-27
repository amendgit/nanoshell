use file_open_dialog::FileOpenDialogService;
use nanoshell::{
    codec::Value,
    shell::{Context, ContextOptions},
};

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod file_open_dialog;

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let context = Context::new(ContextOptions {
        app_namespace: "NanoshellDemo".into(),
        ..Default::default()
    });

    let context = context.unwrap();

    let _file_open_dialog = FileOpenDialogService::new(context.clone());

    context
        .window_manager
        .borrow_mut()
        .create_window(Value::Null, None);

    context.run_loop.borrow().run();
}
