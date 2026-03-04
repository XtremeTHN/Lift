use gtk4::{
    CompositeTemplate, glib, prelude::WidgetExt, subclass::prelude::*
};
use libadwaita::{NavigationPage, subclass::prelude::*};
use glib::subclass::InitializingObject;

mod imp {
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
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {}
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk4::Widget, NavigationPage,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl RomsPage {
    async fn clear_all(&self) {
        let obj = self.imp();
        while let Some(child) = obj.list_box.first_child() {
            obj.list_box.remove(&child);
        }
    }
}