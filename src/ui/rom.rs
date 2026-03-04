use std::{default, path::Path};

use gtk4::{
    CompositeTemplate, ListBoxRow, gio::{self, File, prelude::FileExt}, glib::{self, object::Cast, variant::ToVariant}, prelude::WidgetExt, subclass::prelude::*
};

use glib::subclass::InitializingObject;

mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/roms-page.ui")]
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

            // klass.install_action("row.", parameter_type, activate);
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
    async fn fallback(&self, file: &Path) {
        let f = File::for_path(file);
        let i = f.query_info_future("standard::size,standard::name", gio::FileQueryInfoFlags::NONE, glib::Priority::DEFAULT).await;
        let obj = self.imp();

        match i {
            Ok(info) => {
                let name = info.name();
                let ext = name.extension();
                let size = glib::format_size_full(info.size() as u64, glib::FormatSizeFlags::DEFAULT);

                if ext.is_none() {
                    obj.rom_version.set_label("Format: unknown");
                } else {
                    obj.rom_version.set_label(&format!("Format: {}", ext.unwrap().to_string_lossy().to_string()));
                }

                obj.rom_title.set_label(&name.to_string_lossy().to_string());
                obj.rom_size.set_label(&format!("Size: {}", size));
            }

            Err(err) => {
                self.activate_action("win.toast", Some(&err.to_string().to_variant())).expect("couldn't send action");
            }
        }
    }

    pub fn from_file(file: &Path) {
        // file.extension().unwrap();
    }

    pub fn handle_xci() {
        gio::spawn_blocking(|| {
            
        });
    }

    pub fn handle_nsp() {
        gio::spawn_blocking(|| {

        });
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