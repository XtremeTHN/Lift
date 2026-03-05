use std::{cell::RefCell, fs::File, path::{Path, PathBuf}};

use gtk4::{
    CompositeTemplate, ListBoxRow, gio, gdk, glib::{self, Object, object::{Cast, IsA}, variant::ToVariant}, prelude::WidgetExt, subclass::prelude::*
};

use glib::subclass::InitializingObject;

use crate::{rom_info::RomInfo, roms::formats::nacp::Nacp};

mod imp {
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/rom.ui")]
    pub struct Rom {
        #[template_child]
        pub frame: TemplateChild<gtk4::Frame>,

        #[template_child]
        pub icon: TemplateChild<gtk4::Picture>,

        #[template_child]
        pub rom_title: TemplateChild<gtk4::Label>,

        #[template_child]
        pub rom_version: TemplateChild<gtk4::Label>,

        #[template_child]
        pub rom_size: TemplateChild<gtk4::Label>,

        #[template_child]
        pub end_button_image: TemplateChild<gtk4::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Rom {
        const NAME: &'static str = "Rom";

        type Type = super::Rom;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Rom {}
    impl WidgetImpl for Rom {}
    impl ListBoxRowImpl for Rom {}
}

glib::wrapper! {
    pub struct Rom(ObjectSubclass<imp::Rom>)
        @extends gtk4::Widget, ListBoxRow,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}


#[gtk4::template_callbacks]
impl Rom {
    fn send_error(&self, message: &str) {
        self.activate_action("win.toast", Some(&message.to_string().to_variant())).expect("toast");
    }

    pub fn new(path: PathBuf) -> Self {
        let o: Rom = Object::builder().build();

        let c = o.clone();
        glib::MainContext::default().spawn_local(async move {
            c.populate(path).await;
        });
        return o;
    }

    fn set_or(&self, widget: &TemplateChild<gtk4::Label>, prefix: String, value: Option<String>, default: &str) {
        if let Some(v) = value {
            widget.set_label(&(prefix + &v));
        } else {
            widget.set_label(&(prefix + &default));
        }
    }

    async fn populate(&self, path: PathBuf) {
        let (sender, reciever) = async_channel::bounded::<(Option<RomInfo>, Option<String>)>(1);

        let send = sender.clone();
        gio::spawn_blocking(move || {
            let rom_info = RomInfo::new(path);
            if let Err(e) = rom_info {
                send.send_blocking((None, Some(e.to_string()))).expect("failed to send data");
                return;
            }

            let mut unwrapped = rom_info.unwrap();
            if let Err(e) = unwrapped.populate() {
                send.send_blocking((None, Some(e.to_string()))).expect("failed to send data");
                return;
            }

            sender.send_blocking((Some(unwrapped), None)).expect("failed to send data");
        });

        match reciever.recv().await {
            Ok((info, error)) => {
                if !error.is_none() {
                    self.send_error(&error.unwrap());
                    // TODO: handle fallback
                    return;
                }

                let rom_info = info.unwrap();
                let obj = self.imp();

                self.set_or(&obj.rom_title, String::new(), rom_info.title, "Unknown");
                self.set_or(&obj.rom_version, String::from("Version: "), rom_info.version, "0.0.0");

                if let Some(image_data) = rom_info.image_data {
                    let bytes = glib::Bytes::from(&image_data);
                    let texture = gdk::Texture::from_bytes(&bytes);

                    match texture {
                        Ok(t) => {
                            obj.icon.set_paintable(Some(&t));
                        }
                        Err(e) => {
                            self.send_error(&format!("Couldn't construct texture: {}", e.to_string()));
                        }
                    }
                }

                // TODO: handle image fallback
            }
            Err(e) => {
                self.send_error(&e.to_string());
            }
        };
    }

    #[template_callback]
    fn remove_rom(&self, _: &gtk4::Button) {
        if let Some(parent) = self.parent() {
            if let Ok(listbox) = parent.downcast::<gtk4::ListBox>() {
                listbox.remove(self);
            }
        }
    }
}
