use gtk4::{
    CompositeTemplate,
    gio::{
        self,
        prelude::{FileExt, ListModelExt, ListModelExtManual},
    },
    glib::{self, object::{Cast, CastNone, ObjectExt}, variant::ToVariant},
    prelude::WidgetExt,
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;
use libadwaita::{NavigationPage, subclass::prelude::*};

use crate::ui::rom::Rom;

mod imp {
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

            // obj.setup
        }
    }

    impl ObjectImpl for RomsPage {
        fn constructed(&self) {
            self.parent_constructed();
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
        if let Err(e) = wrapped_cast {
            self.activate_action("win.toast", Some(&"Couldn't get window".to_variant()))
                .expect("failed");
            return;
        }

        let cast = wrapped_cast.unwrap();
        let res = diag.open_multiple_future(Some(&cast)).await;

        if let Err(e) = res {
            self.activate_action(
                "win.toast",
                Some(&format!("Couldn't get opened files: {}", e.to_string()).to_variant()),
            )
            .expect("failed");

            return;
        }

        let files = res.unwrap();
        println!("{}", files.n_items());

        let obj = self.imp();
        for i in 0..files.n_items() {
            let x = files.item(i).and_downcast::<gio::File>();
            match x {
                Some(f) => {
                    let path = f.path();

                    if path.is_none() {
                        return;
                    }
                    
                    let row = Rom::new(path.unwrap());
                    obj.list_box.append(&row);
                }
                None => {
                    log::error!("File is None");
                }
            }
        }

        obj.stack.set_visible_child_name("roms");
    }

    async fn clear_all(&self) {
        let obj = self.imp();
        while let Some(child) = obj.list_box.first_child() {
            obj.list_box.remove(&child);
        }
    }
}
