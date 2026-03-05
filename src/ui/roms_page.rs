use gtk4::{
    CompositeTemplate,
    gio::{
        self, ListModel, prelude::{FileExt, ListModelExt, ListModelExtManual}
    },
    glib::{self, Object, object::{Cast, CastNone, ObjectExt}, variant::ToVariant},
    prelude::WidgetExt,
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;
use libadwaita::{NavigationPage, subclass::prelude::*};

use gtk4::glib::types::StaticType;
use crate::{ui::rom::Rom, utils::send_error};

mod imp {

    use gtk4::gio::ListStore;
    use std::cell::RefCell;

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

        pub store: RefCell<Option<ListStore>>
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
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {
        fn constructed(&self) {
            self.parent_constructed();

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

impl RomsPage {
    fn setup_list(&self) {
        let store = gio::ListStore::with_type(gio::File::static_type());

        let obj = self.clone();
        let imp = self.imp();

        imp.store.replace(Some(store.clone()));

        let _obj = obj.clone();
        imp.list_box.bind_model(Some(&store), move |object| {
            let f = object.clone().downcast::<gio::File>();
            let imp = _obj.imp();

            let rom = Rom::new();
            rom.set_file(Some(f.unwrap()));
            rom.set_store(Some(imp.store.borrow().clone().unwrap()));
            rom.populate_sync();

            rom.upcast::<gtk4::Widget>()
        });

        let s = store.clone();
        store.connect_closure("items-changed", true, glib::closure_local!(move |_: ListModel, _: u32, removed: u32, _: u32| {
            if removed > 0 && s.n_items() == 0 {
                obj.imp().stack.set_visible_child_name("placeholder");
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
        if let Err(_) = wrapped_cast {
            send_error(self, "Couldn't get window");
            return;
        }

        let cast = wrapped_cast.unwrap();
        let res = diag.open_multiple_future(Some(&cast)).await;

        if let Err(e) = res {
            if e.to_string() == "Dismissed by user" {
                return;
            }
            send_error(self, &format!("Couldn't get opened files: {}", e.to_string()));
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

                    obj.store.borrow().clone().unwrap().append(&f);
                    obj.stack.set_visible_child_name("roms");
                }
                None => {
                    log::error!("File is None");
                }
            }
        }
    }

    async fn clear_all(&self) {
        let obj = self.imp();
        while let Some(child) = obj.list_box.first_child() {
            obj.list_box.remove(&child);
        }

        obj.stack.set_visible_child_name("placeholder");
    }
}
