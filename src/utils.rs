use gtk4::{
    glib::{object::IsA, variant::ToVariant},
    prelude::WidgetExt,
};

pub fn send_error<W: IsA<gtk4::Widget>>(widget: &W, message: &str) {
    widget
        .activate_action("win.toast", Some(&message.to_string().to_variant()))
        .expect("toast");
}
