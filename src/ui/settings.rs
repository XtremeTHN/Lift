use gtk::{
    gio,
    glib::{self, Object},
    subclass::prelude::*,
};

mod imp {
    use adw::{
        prelude::*,
        subclass::{dialog::AdwDialogImpl, prelude::PreferencesDialogImpl},
    };
    use gtk::prelude::EditableExt;
    use std::cell::OnceCell;
    use std::path::PathBuf;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/settings.ui")]
    pub struct LiftSettings {
        #[template_child]
        lang_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        path_row: TemplateChild<adw::EntryRow>,
        settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LiftSettings {
        const NAME: &'static str = "LiftSettings";
        type Type = super::LiftSettings;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftSettings {
        fn constructed(&self) {
            self.parent_constructed();

            let settings = gio::Settings::new("com.github.XtremeTHN.Lift");
            settings
                .bind("keys-path", &*self.path_row, "title")
                .get()
                .build();

            let lang = settings.enum_("language");
            self.lang_row.set_selected(lang as u32);

            let _ = self.settings.set(settings);
        }
    }
    impl WidgetImpl for LiftSettings {}
    impl AdwDialogImpl for LiftSettings {}
    impl PreferencesDialogImpl for LiftSettings {}

    #[gtk::template_callbacks]
    impl LiftSettings {
        fn show_error(&self, msg: String) {
            let toast = adw::Toast::new(&msg);
            self.obj().add_toast(toast);
        }

        #[template_callback]
        fn on_title_language_changed(&self, _: glib::ParamSpec, row: adw::ComboRow) {
            if let Some(settings) = self.settings.get()
                && let Err(e) = settings.set_enum("language", row.selected() as i32)
            {
                self.show_error(e.to_string());
            };
        }

        #[template_callback]
        fn on_path_changed(&self, row: adw::EntryRow) {
            let text = row.text();
            let path = PathBuf::from(text.clone());

            if !path.exists() {
                self.show_error(format!("\"{}\" does not exist", path.to_string_lossy()));
                return;
            }

            if !path.is_file() {
                self.show_error(format!("\"{}\" is not a file", path.to_string_lossy()));
                return;
            }

            if path.extension().and_then(|e| e.to_str()) != Some("keys") {
                self.show_error("Invalid keys file. Must have a '.keys' extension".to_string());
                return;
            }

            if let Some(settings) = self.settings.get()
                && let Err(e) = settings.set_string("keys-path", &text)
            {
                self.show_error(e.to_string());
                row.add_css_class("warning");
            };
        }
    }
}

glib::wrapper! {
    pub struct LiftSettings(ObjectSubclass<imp::LiftSettings>)
        @extends adw::Dialog, adw::PreferencesDialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl LiftSettings {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
