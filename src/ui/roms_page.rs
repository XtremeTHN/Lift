use async_channel::Receiver;
use gtk4::{
    CompositeTemplate, gdk,
    gio::{
        self, ListModel,
        prelude::{CancellableExt, FileExt, ListModelExt, SettingsExt},
    },
    glib::{
        self,
        object::{Cast, ObjectExt},
    },
    prelude::WidgetExt,
    subclass::prelude::*,
};
use std::rc::Rc;

use glib::subclass::InitializingObject;
use libadwaita::{NavigationPage, subclass::prelude::*};

use crate::{
    ui::rom::Rom,
    usb::{
        self,
        async_protocol::{SwitchProtocol, UsbOperation},
        manager::{Backend, UsbBackend, UsbBackendErrors},
    },
    utils::send_error,
};
use gtk4::glib::types::StaticType;

mod imp {

    use gtk4::{
        gio::{Cancellable, ListStore},
        glib::SourceId,
    };

    use std::cell::{OnceCell, RefCell};
    use std::rc::Rc;

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/roms-page.ui")]
    pub struct RomsPage {
        #[template_child]
        pub rev: TemplateChild<gtk4::Revealer>,

        #[template_child]
        pub info_label: TemplateChild<gtk4::Label>,

        #[template_child]
        pub total_progress: TemplateChild<gtk4::ProgressBar>,

        #[template_child]
        pub stack: TemplateChild<gtk4::Stack>,

        #[template_child]
        pub list_box: TemplateChild<gtk4::ListBox>,

        #[template_child]
        pub top_button_stack: TemplateChild<gtk4::Stack>,

        pub store: OnceCell<ListStore>,
        pub _settings: OnceCell<gio::Settings>,

        pub backend: OnceCell<Rc<Backend>>,
        pub pulse_id: RefCell<Option<SourceId>>,
        pub total_size: RefCell<u64>,
        pub sent_bytes: RefCell<u64>,
        pub cancellable: RefCell<Option<Cancellable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RomsPage {
        const NAME: &'static str = "RomsPage";

        type Type = super::RomsPage;
        type ParentType = NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async("clear-all", None, async |page, _, _| {
                page.clear_all().await;
            });

            klass.install_action_async("open", None, async |page, _, _| {
                page.open_rom().await;
            });

            klass.install_action_async("upload", None, async |page, _, _| {
                if let Err(e) = page.upload().await {
                    send_error(
                        &page.imp().list_box.clone(),
                        &format!("Couldn't upload: {}", e),
                    );
                };
            });

            klass.install_action_async("cancel-upload", None, async |page, _, _| {
                page.cancel_upload();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.store
                .set(gio::ListStore::with_type(gio::File::static_type()))
                .unwrap();

            self.obj().setup_list();
            self.setup_settings();
            self.setup_drag_and_drop();
        }
    }
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}

    impl RomsPage {
        fn setup_settings(&self) {
            let settings = gio::Settings::new("com.github.XtremeTHN.Lift");
            let _ = self._settings.set(settings);
        }

        fn setup_drag_and_drop(&self) {
            let formats = gdk::ContentFormats::for_type(gdk::FileList::static_type());
            // let target = gtk4::DropTarget::new(_type, gdk::DragAction::COPY);
            let target = gtk4::DropTarget::builder()
                .formats(&formats)
                .actions(gdk::DragAction::COPY)
                .build();

            target.connect_enter(|_, _x, _y| gdk::DragAction::COPY);
            target.connect_motion(|_, _x, _y| gdk::DragAction::COPY);

            let obj = self.obj().clone();
            target.connect_drop(move |_, values, _x, _y| {
                match values.get::<gdk::FileList>() {
                    Ok(list) => {
                        for x in list.files() {
                            let path = x.path().unwrap();
                            let extension = path.extension().and_then(|e| e.to_str());
                            if extension != Some("nsp") && extension != Some("xci") {
                                send_error(
                                    &obj,
                                    &format!("File {} is not a rom", path.to_string_lossy()),
                                );
                                continue;
                            }

                            obj.append_to_store(x);
                        }
                    }
                    Err(e) => {
                        send_error(&obj, &format!("Couldn't get dropped files: {:?}", e));
                    }
                }
                true
            });

            self.stack.add_controller(target);
        }
    }
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk4::Widget, NavigationPage,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

#[derive(thiserror::Error, Debug)]
enum UploadErrors {
    #[error("Switch device is none")]
    SwitchNone,
    #[error("LibUsb error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("{0}")]
    Backend(#[from] UsbBackendErrors),
    #[error("Usb protocol error: {0}")]
    Protocol(#[from] usb::async_protocol::ProtocolError),
}

impl RomsPage {
    pub fn set_backend(&self, id: Rc<Backend>) {
        let imp = self.imp();
        let _ = imp.backend.set(id);
    }

    pub fn cancel_upload(&self) {
        if let Some(cancellable) = self.imp().cancellable.borrow().clone() {
            cancellable.cancel();
            self.imp().top_button_stack.set_sensitive(false);
        }
    }

    fn setup_list(&self) {
        let obj = self.clone();
        let imp = self.imp();

        let store = imp.store.get().unwrap();

        let _obj = obj.clone();
        imp.list_box.bind_model(Some(store), move |object| {
            let f = object.clone().downcast::<gio::File>();
            let imp = _obj.imp();

            let rom = Rom::new();
            rom.set_file(Some(f.unwrap()));
            rom.set_store(Some(imp.store.get().unwrap().clone()));

            let settings = imp._settings.get().unwrap();
            if let Err(e) = rom.set_language(settings.enum_("language")) {
                send_error(&_obj, &e.to_string());
            }

            let r = rom.clone();
            glib::MainContext::default().spawn_local(async move {
                r.populate().await;
            });

            rom.upcast::<gtk4::Widget>()
        });

        let s = store.clone();
        store.connect_closure(
            "items-changed",
            true,
            glib::closure_local!(move |_: ListModel, _: u32, removed: u32, _: u32| {
                if removed > 0 && s.n_items() == 0 {
                    obj.imp().stack.set_visible_child_name("placeholder");
                    obj.imp().top_button_stack.set_sensitive(false);
                    obj.action_set_enabled("clear-all", false);
                }
            }),
        );
    }

    fn append_to_store(&self, f: gio::File) {
        let imp = self.imp();
        imp.store.get().unwrap().append(&f);
        imp.stack.set_visible_child_name("roms");
        imp.top_button_stack.set_sensitive(true);
        self.action_set_enabled("clear-all", true);
    }

    async fn open_rom(&self) {
        let filter = gtk4::FileFilter::new();
        filter.add_suffix("xci");
        filter.add_suffix("nsp");

        let diag = gtk4::FileDialog::builder()
            .accept_label("Open")
            .default_filter(&filter)
            .build();

        let r = self.root().unwrap();
        let wrapped_cast = r.downcast::<gtk4::Window>();
        if wrapped_cast.is_err() {
            send_error(self, "Couldn't get window");
            return;
        }

        let cast = wrapped_cast.unwrap();
        let res = diag.open_multiple_future(Some(&cast)).await;

        if let Err(e) = res {
            if e.to_string() == "Dismissed by user" {
                return;
            }
            send_error(self, &format!("Couldn't get opened files: {}", e));
            return;
        }

        let files = res.unwrap();

        let obj = self.clone();
        self.iterate_model(files, |f, _| {
            obj.append_to_store(f);
            true
        });
    }

    fn get_rom(&self, name: &str) -> Option<Rom> {
        let imp = self.imp();

        let mut result = None::<Rom>;
        self.iterate_store(|file, i| {
            let path = file.path().unwrap().to_string_lossy().to_string();

            if path != name {
                return true;
            }

            let r = imp.list_box.row_at_index(i as i32).unwrap();
            let rom = r.downcast::<Rom>();
            if let Ok(unwrapped) = rom {
                result = Some(unwrapped);
                false
            } else {
                result = None;
                false
            }
        });

        result
    }

    async fn send_roms(&self, ctx: &SwitchProtocol) -> Result<(), UploadErrors> {
        let mut files: Vec<String> = vec![];
        self.iterate_store(|file, _| {
            files.push(file.path().unwrap().to_string_lossy().to_string());
            true
        });

        ctx.send_roms(files).await?;
        Ok(())
    }

    fn iterate_model<F: FnMut(gio::File, u32) -> bool>(&self, model: ListModel, mut func: F) {
        for x in 0..model.n_items() {
            if let Some(obj) = model.item(x) {
                let f = obj.downcast::<gio::File>();
                match f {
                    Ok(file) => {
                        if !func(file, x) {
                            break;
                        }
                    }
                    Err(_) => {
                        log::warn!("Couldn't cast file in position {}. Ignoring rom...", x);
                    }
                }
            }
        }
    }

    fn iterate_store<F: FnMut(gio::File, u32) -> bool>(&self, func: F) {
        let imp = self.imp();
        let store = imp.store.get().unwrap();
        self.iterate_model(store.clone().upcast::<gio::ListModel>(), func);
    }

    fn reset_state(&self) {
        let imp = self.imp();
        self.set_pulse(false);
        imp.rev.set_reveal_child(false);
        imp.total_progress.set_fraction(0.0);
        imp.total_size.replace(0);
        imp.sent_bytes.replace(0);

        imp.top_button_stack.set_sensitive(true);
        imp.top_button_stack.set_visible_child_name("upload");
        self.action_set_enabled("upload", true);
        self.iterate_store(|_, index| {
            let wrapped_row = imp.list_box.row_at_index(index as i32);
            if let Some(row) = wrapped_row
                && let Ok(rom) = row.downcast::<Rom>()
            {
                rom.reset_state();
            }
            true
        });
    }

    fn set_pulse(&self, pulse: bool) {
        let imp = self.imp();
        if pulse {
            let obj = self.clone();
            let t = glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
                obj.imp().total_progress.pulse();
                glib::ControlFlow::Continue
            });

            imp.pulse_id.replace(Some(t));
        } else {
            let old = imp.pulse_id.replace(None);
            if let Some(id) = old {
                id.remove();
            }
        }
    }

    fn add_progress(&self, bytes: u64) {
        let imp = self.imp();

        let sent = imp.sent_bytes.replace_with(|old| *old + bytes);
        let total = *imp.total_size.borrow();

        if total > 0 {
            imp.total_progress.set_fraction(sent as f64 / total as f64);
        }
    }

    fn recieve_callbacks(&self, reciever: Receiver<UsbOperation>) {
        let obj = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let imp = obj.imp();
            imp.rev.set_reveal_child(true);

            while let Some(msg) = reciever.recv().await.iter().next() {
                match msg {
                    UsbOperation::File(name, read) => {
                        obj.set_pulse(false);
                        imp.info_label.set_label(&format!("Sending file: {}", name));
                        let r = obj.get_rom(name);

                        if let Some(r) = r {
                            r.set_show_progress_bar(true);
                            r.add_progress(*read);
                            obj.add_progress(*read);
                        }
                    }
                    UsbOperation::Wait => {
                        imp.info_label.set_label("Waiting for command...");
                        obj.set_pulse(true);
                    }
                    UsbOperation::Exit => {
                        imp.info_label.set_label("Exit recieved");
                        obj.reset_state();
                    }
                }
            }
        });
    }

    fn calculate_total_size(&self) {
        let imp = self.imp();
        self.iterate_store(|_, index| {
            let row = imp.list_box.row_at_index(index as i32).unwrap();

            if let Ok(rom) = row.downcast::<Rom>() {
                imp.total_size
                    .replace_with(|&mut old| old + rom.imp().size.get() as u64);
            }
            true
        });
    }

    async fn upload(&self) -> Result<(), UploadErrors> {
        let backend_wrapped = self.imp().backend.get();
        if backend_wrapped.is_none() {
            return Err(UploadErrors::SwitchNone);
        }

        let mut ctx = backend_wrapped.as_ref().unwrap().device().await?;
        let imp = self.imp();
        self.calculate_total_size();
        self.send_roms(&ctx).await?;
        let (sender, reciever) = async_channel::unbounded();
        self.recieve_callbacks(reciever);

        let cancellable = gio::Cancellable::new();
        imp.cancellable.replace(Some(cancellable.clone()));
        imp.top_button_stack.set_visible_child_name("cancel");
        let res = ctx.poll_commands(Some(cancellable), sender).await;
        self.reset_state();

        if let Err(e) = res {
            return Err(UploadErrors::Protocol(e));
        };

        Ok(())
    }

    async fn clear_all(&self) {
        let obj = self.imp();
        obj.store.get().unwrap().remove_all();
        obj.stack.set_visible_child_name("placeholder");
    }
}
