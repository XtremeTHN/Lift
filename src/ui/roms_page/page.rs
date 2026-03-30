use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use crate::{
        rom_data::RomDataLoader,
        ui::{rom::Rom, window::LiftWindow},
        utils,
    };

    use std::cell::OnceCell;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/roms_page.ui")]
    pub struct RomsPage {
        #[template_child]
        pub rev: TemplateChild<gtk::Revealer>,

        #[template_child]
        pub info_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub total_progress: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,

        #[template_child]
        pub top_button_stack: TemplateChild<gtk::Stack>,

        pub settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RomsPage {
        const NAME: &'static str = "RomsPage";
        type Type = super::RomsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {
        fn constructed(&self) {
            self.parent_constructed();

            let settings = gio::Settings::new("com.github.XtremeTHN.Lift");
            let _ = self.settings.set(settings);
        }
    }
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}

    #[gtk::template_callbacks]
    impl RomsPage {
        #[template_callback]
        fn on_clear_clicked(&self, _: gtk::Button) {
            self.list_box.remove_all();
        }

        #[template_callback]
        async fn on_open_rom_clicked(&self, _: gtk::Button) {
            let filter = gtk::FileFilter::new();
            filter.add_suffix("xci");
            filter.add_suffix("nsp");

            let dialog = gtk::FileDialog::builder()
                .default_filter(&filter)
                .accept_label("Open")
                .build();

            let obj = self.obj();
            if let Some(root) = obj.root()
                && let Ok(w) = root.downcast::<LiftWindow>()
            {
                if let Ok(files) = dialog.open_multiple_future(Some(&w)).await {
                    utils::iterate_model_async(files, async move |file, _| {
                        let rom = Rom::new();

                        let settings = self.settings.get().unwrap();
                        let lang = settings.enum_("language");
                        let keyring_path = settings.string("keys-path");

                        if let Err(e) = rom.populate(file, lang, keyring_path.to_string()).await {
                            utils::send_error(&self.obj().clone(), &e.to_string());
                        };

                        self.list_box.append(&rom);

                        true
                    })
                    .await;

                    self.stack.set_visible_child_name("roms");
                }
            } else {
                utils::send_error(&obj.clone(), "Couldn't get active window");
            }
        }

        #[template_callback]
        fn on_upload_switch_clicked(&self, _: gtk::Button) {}

        #[template_callback]
        fn on_cancel_upload_clicked(&self, _: gtk::Button) {}
    }
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub trait RomsPageImpl: NavigationPageImpl {}

unsafe impl<T: RomsPageImpl> IsSubclassable<T> for RomsPage {}
