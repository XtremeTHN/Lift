use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
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

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>){
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {}
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}
    
    #[gtk::template_callbacks]
    impl RomsPage {
        #[template_callback]
        fn on_clear_clicked(&self, _: gtk::Button) {}

        #[template_callback]
        fn on_open_rom_clicked(&self, _: gtk::Button) {}

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
