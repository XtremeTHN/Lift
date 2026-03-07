use std::{cell::{RefCell, Cell}, path::PathBuf};

use gtk4::{
    CompositeTemplate, ListBoxRow, gdk,
    gio::{
        self, ListStore,
        prelude::{FileExt, ListModelExt},
    },
    glib::{self, Object, object::Cast, property::PropertyGet},
    prelude::{FrameExt, WidgetExt},
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;

use crate::{rom_info::RomInfo, utils::send_error};
use super::circular_progress_paintable::{CircularProgressPaintable, Color};

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
        pub end_button: TemplateChild<gtk4::Button>,

        #[template_child]
        pub button_stack: TemplateChild<gtk4::Stack>,

        #[template_child]
        pub img: TemplateChild<gtk4::Image>,

        #[template_child]
        pub prog_bar: TemplateChild<CircularProgressPaintable>,

        pub current_progress: u64,
        pub size: Cell<i64>,
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

    impl ObjectImpl for Rom {
        fn constructed(&self) {
            self.parent_constructed();
            self.img.set_paintable(Some(&self.prog_bar.clone()));
            self.prog_bar.imp().set_widget(Some(self.img.clone()));
        }
    }
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

    pub fn reset_state(&self) {
        let imp = self.imp();
        self.set_show_progress_bar(false);
        imp.prog_bar.imp().set_progress(0.0);
    }

    async fn setup_size(&self) {
        let imp = self.imp();
        let file = {
            let f =imp.file.borrow();
            f.clone()
        };

        if file.is_none() {
            return;
        }

        let f = file.as_ref().unwrap();

        let querier = f
            .query_info_future(
                "standard::size",
                gio::FileQueryInfoFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await;
            
        if let Ok(s) = querier {
            println!("size {}", s.size());
            imp.size.set(s.size());
        }
    }

    pub fn set_show_progress_bar(&self, show: bool) {
        let imp = self.imp();

        if show {
            imp.button_stack.set_visible_child_name("progress");
            imp.end_button.set_sensitive(false);
        } else {
            imp.button_stack.set_visible_child_name("remove");
            imp.end_button.set_sensitive(true);
        }
    }

    pub fn add_progress(&self, read_size: u64) {
        let imp = self.imp();
        imp.prog_bar.imp().add_progress(read_size as f64 / imp.size.get() as f64);

        if imp.prog_bar.imp().progress.get() == 1.0 {
            imp.prog_bar.imp().set_color(Color::Success);
        }
    }

    pub async fn populate(&self) {
        let imp = self.imp();
        let _path = {
            let f = imp.file.borrow();
            f.as_ref().and_then(|f| f.path())
        };

        if _path.is_none() {
            return;
        }

        let path = _path.unwrap();

        self.setup_size().await;

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
                    &self.format_size(Some(self.imp().size.get())),
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
            &self.format_size(Some(self.imp().size.get())),
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
