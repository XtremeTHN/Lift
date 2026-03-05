use std::sync::LazyLock;

use gtk4::{glib::{object::IsA, variant::ToVariant}, prelude::WidgetExt};

pub static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

pub async fn spawn_tokio<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    RUNTIME.spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.await.unwrap()
}


pub fn send_error<W: IsA<gtk4::Widget>>(widget: &W, message: &str) {
    widget.activate_action("win.toast", Some(&message.to_string().to_variant()))
        .expect("toast");
}