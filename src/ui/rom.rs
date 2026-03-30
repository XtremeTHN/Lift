use crate::{
    rom_data::{FromGFileErrors, HandlingErrors, RomData, RomDataLoader},
    utils,
};
use gtk::{
    gio,
    glib::{self, Object},
    prelude::{FrameExt, WidgetExt},
    subclass::prelude::{ListBoxRowImpl, *},
};
use nxroms::formats::{cnmt::ContentMetaType, nacp::TitleLanguage};

use super::circular_progress_paintable::CircularProgressPaintable;

mod imp {
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
        fn remove_rom(&self) {}
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

    pub async fn populate(
        &self,
        file: gio::File,
        language: i32,
        keyring_path: String,
    ) -> Result<(), RomErrors> {
        let lang = TitleLanguage::try_from(language).expect("language not in range");
        let data = gio::spawn_blocking(move || -> Result<RomData, RomErrors> {
            let loader = RomDataLoader::from_gfile(file, lang, keyring_path)?;
            Ok(loader.load()?)
        })
        .await;

        match data {
            Ok(res) => {
                let data = res?;
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
            }

            Err(_) => return Err(RomErrors::GioThread),
        }

        Ok(())
    }
}
