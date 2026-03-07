use std::os::fd::AsRawFd;

use ashpd::{WindowIdentifier, desktop::usb::{Device, DeviceID, UsbProxy}};
use gtk4::{
    CompositeTemplate,
    gio::{
        self, ListModel, prelude::{FileExt, ListModelExt}
    },
    glib::{self, object::{Cast, CastNone, ObjectExt}},
    prelude::WidgetExt,
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;
use libadwaita::{NavigationPage, subclass::prelude::*};

use gtk4::glib::types::StaticType;
use crate::{ui::rom::Rom, usb::{self, async_protocol::SwitchProtocol}, utils::send_error};

mod imp {

    use gtk4::gio::ListStore;
    use std::cell::{OnceCell, RefCell};

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

        pub store: OnceCell<ListStore>,

        pub switch_id: RefCell<Option<DeviceID>>,
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
                    send_error(&page.imp().list_box.clone(), &format!("Couldn't upload: {}", e));
                };
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.store.set(gio::ListStore::with_type(gio::File::static_type())).unwrap();
            self.obj().setup_list();
        }
    }
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk4::Widget, NavigationPage,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

#[derive(thiserror::Error, Debug)]
enum UploadErrors {
    #[error("couldn't get usb proxy: {0}")]
    UsbProxyGet(#[from] ashpd::Error),
    #[error("portal usb error: {0}")]
    UsbErrorProxy(String),
    #[error("switch device is none")]
    SwitchNone,
    #[error("usb error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("usb protocol error: {0}")]
    Protocol(#[from] usb::async_protocol::ProtocolError)
}

impl RomsPage {
    pub fn set_switch_id(&self, id: Option<DeviceID>) {
        let imp = self.imp();
        imp.switch_id.replace(id);
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
            rom.populate_sync();

            rom.upcast::<gtk4::Widget>()
        });

        let s = store.clone();
        store.connect_closure("items-changed", true, glib::closure_local!(move |_: ListModel, _: u32, removed: u32, _: u32| {
            if removed > 0 && s.n_items() == 0 {
                obj.imp().stack.set_visible_child_name("placeholder");
                obj.action_set_enabled("clear-all", false);
            }
        }));
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

        let obj = self.imp();
        for i in 0..files.n_items() {
            let x = files.item(i).and_downcast::<gio::File>();
            match x {
                Some(f) => {
                    let path = f.path();

                    if path.is_none() {
                        return;
                    }

                    obj.store.get().unwrap().append(&f);
                    obj.stack.set_visible_child_name("roms");
                    self.action_set_enabled("clear-all", true);
                }
                None => {
                    log::error!("File is None");
                }
            }
        }
    }

    async fn send_roms(&self, ctx: &SwitchProtocol) -> Result<(), UploadErrors> {
        let imp = self.imp();

        let mut files: Vec<String> = vec![];

        let store = imp.store.get().unwrap();
        for index in 0..store.n_items() {
            if let Some(obj) = store.item(index) {
                let f = obj.downcast::<gio::File>();
                match f {
                    Ok(file) => {
                        files.push(file.path().unwrap().to_string_lossy().to_string());
                    }
                    Err(_) => {
                        log::warn!("Couldn't cast file in position {}. Ignoring rom...", index);
                    }
                }
            }
        }

        ctx.send_roms(files).await?;
        Ok(())
    }

    async fn upload(&self) -> Result<(), UploadErrors> {
        let proxy = UsbProxy::new().await?;
        let dev_wrapped = {
            let b = self.imp().switch_id.borrow();
            b.clone()
        };

        if dev_wrapped.is_none() {
            return Err(UploadErrors::SwitchNone);
        }

        let dev_id = dev_wrapped.unwrap();
        let root = self.native().unwrap();
        let handle = WindowIdentifier::from_native(&root).await;
        let device = Device::new(dev_id, true);
        let res = proxy.acquire_devices(handle.as_ref(), &[device], Default::default()).await?;
        
        if let Some((_, fd_res)) = res.first() {
            let mut ctx = SwitchProtocol::new()?;
            
            match fd_res {
                Ok(fd) => {
                    ctx.open_switch_from_fd(fd.as_raw_fd()).await?;
                }
                Err(e) => {
                    return Err(UploadErrors::UsbErrorProxy(e.to_string()));
                }
            }

            self.send_roms(&ctx).await?;
            ctx.poll_commands().await?;
        };

        Ok(())
    }

    async fn clear_all(&self) {
        let obj = self.imp();
        obj.store.get().unwrap().remove_all();
        obj.stack.set_visible_child_name("placeholder");
    }
}
