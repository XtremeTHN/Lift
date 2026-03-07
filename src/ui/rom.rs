use std::{cell::RefCell, path::PathBuf};

use gtk4::{
    CompositeTemplate, ListBoxRow, gdk,
    gio::{
        self, ListStore,
        prelude::{FileExt, ListModelExt},
    },
    glib::{self, Object, object::Cast},
    prelude::FrameExt,
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;

use crate::{rom_info::RomInfo, utils::send_error};

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

        pub file: RefCell<Option<gio::File>>,
        pub store: RefCell<Option<ListStore>>,
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
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_store(&self, store: Option<ListStore>) {
        self.imp().store.replace(store);
    }

    pub fn set_file(&self, file: Option<gio::File>) {
        self.imp().file.replace(file);
    }

    pub fn populate_sync(&self) {
        let obj = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let imp = obj.imp();
            let path = {
                let f = imp.file.borrow();
                f.as_ref().and_then(|f| f.path())
            };

            if let Some(p) = path {
                obj.populate(p).await;
            }
        });
    }

    fn set_or(
        &self,
        widget: &TemplateChild<gtk4::Label>,
        prefix: &str,
        value: &Option<String>,
        default: &str,
    ) {
        if let Some(v) = value {
            widget.set_label(&format!("{}{}", prefix, v));
        } else {
            widget.set_label(&format!("{}{}", prefix, default));
        }
    }

    fn format_size(&self, size: Option<i64>) -> Option<String> {
        if let Some(s) = size {
            let fmtd = glib::format_size(s.clamp(0, i64::MAX) as u64);
            Some(fmtd.to_string())
        } else {
            None
        }
    }

    pub async fn size(&self, path: &PathBuf) -> Option<i64> {
        let f = gio::File::for_path(path);

        let querier = f
            .query_info_future(
                "standard::size",
                gio::FileQueryInfoFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await;
        if let Ok(s) = querier {
            Some(s.size())
        } else {
            None
        }
    }

    pub async fn populate(&self, path: PathBuf) {
        let (sender, reciever) = async_channel::bounded::<(Option<RomInfo>, Option<String>)>(1);

        let send = sender.clone();
        let _path = path.clone();
        gio::spawn_blocking(move || {
            let rom_info = RomInfo::new(_path);
            if let Err(e) = rom_info {
                send.send_blocking((None, Some(e.to_string())))
                    .expect("failed to send data");
                return;
            }

            let mut unwrapped = rom_info.unwrap();
            if let Err(e) = unwrapped.populate() {
                send.send_blocking((None, Some(e.to_string())))
                    .expect("failed to send data");
                return;
            }

            sender
                .send_blocking((Some(unwrapped), None))
                .expect("failed to send data");
        });

        let obj = self.imp();
        match reciever.recv().await {
            Ok((info, error)) => {
                if let Some(err) = error {
                    send_error(self, &err);
                    self.set_default_data(path).await;
                    return;
                }

                let rom_info = info.unwrap();

                self.set_or(&obj.rom_title, "", &rom_info.title, "Unknown");
                self.set_or(&obj.rom_version, "Version: ", &rom_info.version, "0.0.0");
                self.set_or(
                    &obj.rom_size,
                    "Size: ",
                    &self.format_size(self.size(&path).await),
                    "0b",
                );

                if let Some(image_data) = rom_info.image_data {
                    let bytes = glib::Bytes::from(&image_data);
                    let texture = gdk::Texture::from_bytes(&bytes);

                    match texture {
                        Ok(t) => {
                            obj.icon.set_paintable(Some(&t));
                        }
                        Err(e) => {
                            send_error(
                                self,
                                &format!("Couldn't construct texture: {}", e),
                            );
                        }
                    }
                }

                // TODO: handle image fallback
            }
            Err(e) => {
                send_error(self, &e.to_string());
                self.set_default_data(path).await;
            }
        };
    }

    async fn set_default_data(&self, path: PathBuf) {
        let obj = self.imp();
        obj.rom_title
            .set_label(path.file_name().unwrap().to_string_lossy().as_ref());
        obj.rom_version.set_label("Version: 0.0.0");

        let img = gtk4::Image::builder()
            .icon_name("image-missing-symbolic")
            .pixel_size(60)
            .build();
        obj.frame.set_child(Some(&img));

        self.set_or(
            &obj.rom_size,
            "Size: ",
            &self.format_size(self.size(&path).await),
            "0b",
        );
    }

    #[template_callback]
    fn remove_rom(&self, _: &gtk4::Button) {
        let imp = self.imp();

        let wrapped_file = imp.file.borrow();
        let f = wrapped_file.as_ref().unwrap();
        let wrapped_store = imp.store.borrow();
        let s = wrapped_store.as_ref().unwrap();

        for i in 0..s.clone().n_items() {
            if let Some(item) = s.item(i)
                && item.downcast_ref::<gio::File>() == Some(f) {
                    s.remove(i);
                    break;
                }
        }
    }
}
