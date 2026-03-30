use gtk::{
    gio::{self, prelude::ListModelExt},
    glib::{object::IsA, variant::ToVariant},
    prelude::WidgetExt,
};

use adw::prelude::Cast;
use core::future::Future;

pub fn send_error<W: IsA<gtk::Widget>>(widget: &W, message: &str) {
    widget
        .activate_action("win.toast", Some(&message.to_string().to_variant()))
        .expect("toast");
}

pub fn iterate_model<F: FnMut(gio::File, u32) -> bool>(model: gio::ListModel, mut func: F) {
    for x in 0..model.n_items() {
        if let Some(obj) = model.item(x) {
            let f = obj.downcast::<gio::File>();
            match f {
                Ok(file) => {
                    if !func(file, x) {
                        break;
                    }
                }
                Err(_) => {
                    log::warn!("Couldn't cast file in position {}. Ignoring rom...", x);
                }
            }
        }
    }
}

pub async fn iterate_model_async<Fut, F>(model: gio::ListModel, mut func: F)
where
    Fut: Future<Output = bool>,
    F: FnMut(gio::File, u32) -> Fut,
{
    for x in 0..model.n_items() {
        if let Some(obj) = model.item(x) {
            let f = obj.downcast::<gio::File>();
            match f {
                Ok(file) => {
                    if !func(file, x).await {
                        break;
                    }
                }
                Err(_) => {
                    log::warn!("Couldn't cast file in position {}. Ignoring rom...", x);
                }
            }
        }
    }
}
