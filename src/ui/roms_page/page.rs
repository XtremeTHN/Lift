use std::collections::HashMap;

use adw::subclass::prelude::*;
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::rom::Rom;

mod imp {
    use gtk::glib::subclass::Signal;

    use crate::{
        ui::{rom::Rom, window::LiftWindow},
        utils,
    };

    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/roms_page.ui")]
    pub struct RomsPage {
        #[template_child]
        pub rev: TemplateChild<gtk::Revealer>,

        #[template_child]
        pub info_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,

        #[template_child]
        pub top_button_stack: TemplateChild<gtk::Stack>,

        pub settings: OnceCell<gio::Settings>,
        pub pulse_task: RefCell<Option<glib::SourceId>>,
        pub current_progress: RefCell<f64>,
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

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("upload-clicked").build(),
                    Signal::builder("cancel-clicked").build(),
                ]
            })
        }
    }
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}
    impl RomsPageImpl for RomsPage {}

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

                        match rom.populate(file, lang, keyring_path.to_string()).await {
                            Err(e) => {
                                utils::send_error(&self.obj().clone(), &e.to_string());
                                return true;
                            }
                            Ok(Some(e)) => {
                                utils::send_error(
                                    &*self.obj(),
                                    &format!("Falling back to defaults: {}", e.to_string()),
                                );
                            }
                            Ok(None) => {}
                        };

                        self.list_box.append(&rom);

                        true
                    })
                    .await;
                }
            } else {
                utils::send_error(&obj.clone(), "Couldn't get active window");
            }
        }

        #[template_callback]
        fn on_upload_switch_clicked(&self, _: gtk::Button) {
            self.obj().emit_by_name::<()>("upload-clicked", &[]);
        }

        #[template_callback]
        fn on_cancel_upload_clicked(&self, _: gtk::Button) {
            self.obj().emit_by_name::<()>("cancel-clicked", &[]);
        }
    }
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RomsPage {
    fn iterate_rows<F: FnMut(Rom, i32)>(&self, mut cb: F) {
        let imp = self.imp();
        let Some(first) = imp.list_box.first_child() else {
            return;
        };

        let Some(mut rom) = first.downcast::<Rom>().ok() else {
            return;
        };

        cb(rom.clone(), 0);

        let mut index = 0;
        loop {
            let next_row = rom.next_sibling();
            index += 1;

            if let Some(widget) = next_row
                && let Some(row) = widget.downcast::<Rom>().ok()
            {
                cb(row.clone(), index);
                rom = row;
            } else {
                break;
            }
        }
    }

    pub fn all_rows(&self) -> Option<Vec<Rom>> {
        let mut vec: Vec<Rom> = vec![];

        self.iterate_rows(|row, _| {
            vec.push(row);
        });

        Some(vec)
    }

    pub fn roms_hash(&self) -> HashMap<String, Rom> {
        let mut hash: HashMap<String, Rom> = HashMap::new();
        self.iterate_rows(|r, _| {
            let path = r.path();
            let Some(name) = path.file_name() else {
                return;
            };

            hash.insert(name.to_string_lossy().to_string(), r.clone());
        });

        hash
    }

    pub fn rom(&self, file_name: &str) -> Option<Rom> {
        let mut rom = None;
        self.iterate_rows(|r, _| {
            let path = r.path();

            if let Some(name) = path.file_name()
                && name == file_name
            {
                rom = Some(r);
            }
        });

        rom
    }

    pub fn total_size(&self, rows: &Vec<Rom>) -> i64 {
        rows.iter().map(|r| r.size()).sum()
    }

    pub fn add_progress(&self, bytes: i64, total_size: i64) {
        let imp = self.imp();
        let old = imp.current_progress.borrow().clone();
        let new = (bytes as f64 / total_size as f64) + old;

        imp.current_progress.replace(new);
        imp.progress_bar.set_fraction(new);
    }

    pub fn reset_state(&self) {
        let imp = self.imp();

        imp.current_progress.replace(0.0);
        imp.progress_bar.set_fraction(0.0);

        self.set_pulse(false);
        self.set_cancel_visible(false);
        self.set_info_reveal(false);

        self.iterate_rows(|r, _| {
            r.reset_state();
        });
    }

    pub fn set_info(&self, msg: &str) {
        self.imp().info_label.set_label(msg);
    }

    pub fn set_pulse(&self, pulse: bool) {
        let imp = self.imp();
        // ControlFlow::
        if pulse {
            imp.pulse_task.replace(Some(glib::timeout_add_local(
                std::time::Duration::from_millis(400),
                glib::clone!(
                    #[weak]
                    imp,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move || {
                        imp.progress_bar.pulse();
                        ControlFlow::Continue
                    }
                ),
            )));
        } else {
            if let Some(old) = imp.pulse_task.take() {
                old.remove();
            }
        }
    }

    pub fn set_cancel_visible(&self, visible: bool) {
        self.imp()
            .top_button_stack
            .set_visible_child_name(if visible { "cancel" } else { "upload" });
    }

    pub fn set_info_reveal(&self, reveal: bool) {
        self.imp().rev.set_reveal_child(reveal);
    }
}

pub trait RomsPageImpl: NavigationPageImpl {}

unsafe impl<T: RomsPageImpl> IsSubclassable<T> for RomsPage {}
