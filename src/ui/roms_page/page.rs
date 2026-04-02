use std::collections::HashMap;

use crate::usb::async_protocol::ProtocolOperation;

use adw::subclass::prelude::*;
use async_std::channel::Receiver;
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::rom::Rom;

#[derive(Clone, Debug, Default, glib::Boxed)]
#[boxed_type(name = "RomVec")]
pub struct RomVec(pub Vec<Rom>);

#[derive(Clone, Debug, Default, glib::Boxed)]
#[boxed_type(name = "FileVec")]
pub struct FileVec(pub Vec<gio::File>);

mod imp {
    use gtk::glib::subclass::Signal;

    use crate::{
        ui::{rom::Rom, window::LiftWindow},
        utils::{self, CancellableAsyncTasks},
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

        #[template_child]
        pub placeholder: TemplateChild<adw::StatusPage>,

        #[template_child]
        pub clear_all_btt: TemplateChild<gtk::Button>,

        #[template_child]
        pub open_rom_btt: TemplateChild<gtk::Button>,

        pub tasks: RefCell<CancellableAsyncTasks<()>>,

        pub settings: OnceCell<gio::Settings>,
        pub pulse_task: RefCell<Option<glib::SourceId>>,
        pub current_progress: RefCell<f64>,

        pub binding: OnceCell<glib::Binding>,
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

            self.placeholder.connect_map(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.top_button_stack.set_sensitive(false);
                    imp.clear_all_btt.set_sensitive(false);
                    // imp
                }
            ));

            self.placeholder.connect_unmap(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.top_button_stack.set_sensitive(true);
                    imp.clear_all_btt.set_sensitive(true);
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("upload-clicked")
                        .param_types([
                            RomVec::static_type(),
                            FileVec::static_type(),
                            i64::static_type(),
                        ])
                        .build(),
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
            if let Some(rows) = self.obj().all_rows() {
                for x in rows {
                    self.list_box.remove(&x);
                }
            }
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
                                    &format!("Falling back to defaults: {}", e),
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
            let Some(rows) = self.obj().all_rows() else {
                return;
            };

            // TODO: optimize this
            // maybe use only one for loop?
            let roms = RomVec(rows);
            let files = FileVec(
                roms.0
                    .iter()
                    .map(|r| r.file().clone())
                    .collect::<Vec<gio::File>>(),
            );

            let total_size = self.obj().total_size(&roms.0);

            self.obj()
                .emit_by_name::<()>("upload-clicked", &[&roms, &files, &total_size]);
        }

        #[template_callback]
        fn on_cancel_upload_clicked(&self, _: gtk::Button) {
            let mut t = self.tasks.borrow_mut();
            t.cancel_all();
            self.obj().emit_by_name::<()>("cancel-clicked", &[]);
            self.obj().reset_state();
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

    pub fn connect_cancel_clicked<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "cancel-clicked",
            false,
            glib::closure_local!(move |page: RomsPage| { f(&page) }),
        )
    }

    pub fn connect_upload_clicked<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, RomVec, FileVec, i64) + 'static,
    {
        self.connect_closure(
            "upload-clicked",
            false,
            glib::closure_local!(move |page: RomsPage,
                                       roms: RomVec,
                                       files: FileVec,
                                       total_size: i64| {
                f(&page, roms, files, total_size)
            }),
        )
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
            hash.insert(path.to_string_lossy().to_string(), r.clone());
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

    pub fn total_size(&self, rows: &[Rom]) -> i64 {
        rows.iter().map(|r| r.size()).sum()
    }

    pub fn add_progress(&self, bytes: i64, total_size: i64) {
        let imp = self.imp();
        let old = *imp.current_progress.borrow();
        let new = (bytes as f64 / total_size as f64) + old;

        imp.current_progress.replace(new);
        imp.progress_bar.set_fraction(new);
    }

    async fn message_with_timeout(
        &self,
        receiver: &Receiver<ProtocolOperation>,
        secs: u64,
    ) -> Option<ProtocolOperation> {
        match glib::future_with_timeout(std::time::Duration::from_secs(secs), receiver.recv()).await
        {
            Ok(Ok(msg)) => Some(msg),
            Ok(Err(_)) => None, // channel closed
            Err(_) => {
                crate::utils::send_error(self, "Timeout");
                self.emit_by_name::<()>("cancel-upload", &[]);
                None
            }
        }
    }

    async fn message(&self, receiver: &Receiver<ProtocolOperation>) -> Option<ProtocolOperation> {
        receiver.recv().await.ok()
    }

    pub async fn receive_events(
        &self,
        file_timeout: bool,
        receiver: Receiver<ProtocolOperation>,
        total_size: i64,
    ) {
        let hash: HashMap<String, Rom> = self.roms_hash();
        let imp = self.imp();

        let mut with_timeout = false;

        loop {
            let msg = if with_timeout {
                self.message_with_timeout(&receiver, 5).await
            } else {
                self.message(&receiver).await
            };

            match msg {
                Some(ProtocolOperation::File(name, chunk_read)) => {
                    if !with_timeout && file_timeout {
                        with_timeout = true
                    };

                    self.set_pulse(false);
                    imp.info_label.set_label(&format!("Sending {}...", name));
                    self.add_progress(chunk_read as i64, total_size);
                    if let Some(rom) = hash.get(&*name) {
                        rom.set_progress_visible(true);
                        rom.add_progress(chunk_read as i64);
                    } else {
                        crate::utils::send_error(self, &format!("Row not found for rom: {}", name));
                    }
                }
                Some(ProtocolOperation::Wait(message)) => {
                    imp.info_label.set_label(&message);
                    self.set_pulse(true);
                }
                Some(ProtocolOperation::Exit) => {
                    self.reset_state();
                    break;
                }
                None => {
                    break;
                }
            }
        }
    }

    pub fn reset_state(&self) {
        let imp = self.imp();

        imp.current_progress.replace(0.0);
        imp.progress_bar.set_fraction(0.0);

        self.set_pulse(false);
        self.set_cancel_visible(false);
        self.set_info_reveal(false);
        self.set_no_roms(false);

        self.iterate_rows(|r, _| {
            r.reset_state();
        });
    }

    pub fn set_no_roms(&self, no_roms: bool) {
        self.imp().open_rom_btt.set_sensitive(!no_roms);
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
