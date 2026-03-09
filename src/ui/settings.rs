use glib::subclass::InitializingObject;
use gtk4::{CompositeTemplate, TemplateChild, gio, glib::{self, Object}, subclass::prelude::*};
use libadwaita::{Dialog, PreferencesDialog, subclass::prelude::*};

mod imp {
    use std::{cell::OnceCell, path::PathBuf};

    use gtk4::{gio::{prelude::{SettingsExt}}, prelude::{EditableExt, WidgetExt}};
    use libadwaita::prelude::{ComboRowExt, PreferencesDialogExt, PreferencesRowExt};

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/settings.ui")]
    pub struct LiftSettings {
        pub settings: OnceCell<gio::Settings>,
        #[template_child]
        pub lang_row: TemplateChild<libadwaita::ComboRow>,
        #[template_child]
        pub path_row: TemplateChild<libadwaita::EntryRow>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LiftSettings {
        const NAME: &'static str = "LiftSettings";

        type Type = super::LiftSettings;
        type ParentType = PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftSettings {
        fn constructed(&self) {
            self.parent_constructed();
            let settings = gio::Settings::new("com.github.XtremeTHN.Lift");

            self.lang_row.set_selected(settings.enum_("language") as u32);
            self.path_row.set_title(&settings.string("keys-path"));

            let _ = self.settings.set(settings);
        }
    }
    impl WidgetImpl for LiftSettings {}
    impl AdwDialogImpl for LiftSettings {}
    impl PreferencesDialogImpl for LiftSettings {}
    
    #[gtk4::template_callbacks]
    impl LiftSettings {
        fn show_error(&self, msg: String) {
            let toast = libadwaita::Toast::new(&msg);
            self.obj().add_toast(toast);
        }

        #[template_callback]
        fn on_path_changed(&self) {
            let row_text = self.path_row.text();
            let path = PathBuf::from(&row_text);

            if !path.exists() {
                self.show_error(format!("\"{}\" does not exist", path.to_string_lossy()));
                return;
            }

            if !path.is_dir() {
                self.show_error(format!("\"{}\" is not a directory", path.to_string_lossy()));
                return;
            }


            if let Some(settings) = self.settings.get() && let Err(e) = settings.set_string("keys-path", &row_text) {
                self.show_error(e.to_string());
                self.path_row.add_css_class("warning");
            };
        }

        #[template_callback]
        fn on_title_language_changed(&self, _: glib::ParamSpec, row: libadwaita::ComboRow) {
            if let Some(settings) = self.settings.get() && let Err(e) = settings.set_enum("language", row.selected() as i32) {
                self.show_error(e.to_string());
            };
        }
    }
}

glib::wrapper! {
    pub struct LiftSettings(ObjectSubclass<imp::LiftSettings>)
        @extends Dialog, PreferencesDialog, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::ShortcutManager;
}

impl LiftSettings {
    pub fn new() -> Self {
        Object::builder().build()
    }
}