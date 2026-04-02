use std::path::PathBuf;

use crate::{
    rom_data::{FromGFileErrors, HandlingErrors, RomData, RomDataLoader},
    utils,
};
use gtk::{
    gio::{self, prelude::FileExt},
    glib::{self, GString, Object},
    prelude::{FrameExt, WidgetExt},
    subclass::prelude::{ListBoxRowImpl, *},
};
use nxroms::formats::{cnmt::ContentMetaType, nacp::TitleLanguage};

use super::circular_progress_paintable::CircularProgressPaintable;

mod imp {
    use gtk::{ListBox, glib::object::Cast};
    use std::cell::{OnceCell, RefCell};

    use super::*;
    // use self::crate

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/rom.ui")]
    pub struct Rom {
        #[template_child]
        pub frame: TemplateChild<gtk::Frame>,

        #[template_child]
        pub icon: TemplateChild<gtk::Picture>,

        #[template_child]
        pub rom_type_icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub rom_title: TemplateChild<gtk::Label>,

        #[template_child]
        pub rom_version: TemplateChild<gtk::Label>,

        #[template_child]
        pub rom_size: TemplateChild<gtk::Label>,

        #[template_child]
        pub button_stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub img: TemplateChild<gtk::Image>,

        #[template_child]
        pub prog_bar: TemplateChild<CircularProgressPaintable>,

        pub current_progress: RefCell<f64>,
        pub size: RefCell<i64>,
        pub file: OnceCell<gio::File>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Rom {
        const NAME: &'static str = "Rom";
        type Type = super::Rom;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Rom {
        fn constructed(&self) {
            self.prog_bar.imp().set_widget(self.img.clone());
        }
    }
    impl WidgetImpl for Rom {}
    impl ListBoxRowImpl for Rom {}

    #[gtk::template_callbacks]
    impl Rom {
        #[template_callback]
        fn remove_rom(&self) {
            let obj = self.obj();
            let parent = obj.parent().unwrap();

            if let Ok(list) = parent.downcast::<ListBox>() {
                list.remove(&obj.clone());
            }
        }
    }
}

glib::wrapper! {
    pub struct Rom(ObjectSubclass<imp::Rom>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(thiserror::Error, Debug)]
pub enum RomErrors {
    #[error("Invalid file: {0}")]
    InvalidFile(#[from] FromGFileErrors),
    #[error("Couldn't parse rom: {0}")]
    CorruptRom(#[from] HandlingErrors),
    #[error("Unknown error while executing a gio thread")]
    GioThread,
}

impl Rom {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn size(&self) -> i64 {
        *self.imp().size.borrow()
    }

    pub fn add_progress(&self, bytes: i64) {
        let imp = self.imp();
        let old = *imp.current_progress.borrow();
        let new = (bytes as f64 / *imp.size.borrow() as f64) + old;

        imp.current_progress.replace(new);
        imp.prog_bar.imp().set_progress(new);
    }

    pub fn set_progress_visible(&self, visible: bool) {
        let imp = self.imp();

        if visible {
            if imp.button_stack.visible_child_name() != Some(GString::from("progress")) {
                imp.button_stack.set_visible_child_name("progress");
            }
        } else {
            imp.prog_bar.imp().set_progress(0.0);
            imp.current_progress.replace(0.0);
            imp.button_stack.set_visible_child_name("remove");
        }
    }

    pub fn file(&self) -> &gio::File {
        self.imp().file.get().unwrap()
    }

    pub fn path(&self) -> PathBuf {
        self.file().path().unwrap()
    }

    pub fn reset_state(&self) {
        self.set_progress_visible(false);
    }

    pub async fn populate(
        &self,
        file: gio::File,
        language: i32,
        keyring_path: String,
    ) -> Result<Option<HandlingErrors>, RomErrors> {
        self.imp()
            .file
            .set(file.clone())
            .expect("populate() should be called once");

        let lang = TitleLanguage::try_from(language).expect("language not in range");
        let data = gio::spawn_blocking(move || -> Result<RomData, RomErrors> {
            let loader = RomDataLoader::from_gfile(file, lang, keyring_path)?;
            let data = loader.load().unwrap_or_else(|e| loader.load_default(e));
            Ok(data)
        })
        .await
        .map_err(|_| RomErrors::GioThread)
        .flatten()?;

        let imp = self.imp();

        if let Some(data) = data.texture_data {
            imp.icon.set_paintable(Some(&data));
        } else {
            let img = gtk::Image::builder()
                .icon_name("image-missing-symbolic")
                .pixel_size(60)
                .build();
            imp.frame.set_child(Some(&img));
        }

        imp.size.replace(data.size);

        imp.rom_title.set_label(&data.title);
        imp.rom_version
            .set_label(&format!("Version: {}", data.version));

        let fmt_size = glib::format_size(data.size.clamp(0, i64::MAX) as u64);
        imp.rom_size.set_label(&format!("Size: {}", fmt_size));

        match data.meta_type {
            ContentMetaType::Patch => {
                imp.rom_type_icon
                    .set_icon_name(Some("software-update-available"));
                imp.rom_type_icon.set_visible(true);
            }
            ContentMetaType::AddOnContent => {
                imp.rom_type_icon
                    .set_icon_name(Some("application-x-addon-symbolic"));
                imp.rom_type_icon.set_visible(true);
            }
            ContentMetaType::Application => {}
            _ => {
                utils::send_error(
                    self,
                    &format!("Unknown content meta type: {:?}", data.meta_type),
                );
            }
        }

        Ok(data.error)
    }
}
