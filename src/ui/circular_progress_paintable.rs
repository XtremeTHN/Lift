use gtk4::{
    glib,
    gdk,
    graphene,
    subclass::prelude::*,
};

use glib::prelude::ObjectExt;

use std::f64::consts::PI;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Color {
    WHITE = 0,
    ERROR = 1,
    WARNING = 2,
    SUCCESS = 3
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

mod imp {
    use gtk4::{gdk::prelude::PaintableExt, prelude::{SnapshotExt, WidgetExt}};

    use super::*;
    
    #[derive(glib::Properties, Default)]
    #[properties(wrapper_type = super::CircularProgressPaintable)]
    pub struct CircularProgressPaintable {
        pub widget: RefCell<Option<gtk4::Image>>,
        pub color: Color,
        #[property(get, set = Self::set_progress)]
        pub progress: Cell<f64>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CircularProgressPaintable {
        const NAME: &'static str = "CircularProgressPaintable";

        type Type = super::CircularProgressPaintable;
        type ParentType = glib::Object;
        type Interfaces = (gdk::Paintable, gtk4::SymbolicPaintable);
    }

    impl ObjectImpl for CircularProgressPaintable {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl PaintableImpl for CircularProgressPaintable {
        fn snapshot(&self, _: &gdk::Snapshot, _: f64, _: f64) {
            // println!("normal snapshot");
        }

        fn intrinsic_height(&self) -> i32 {
            return 16 * self.widget.borrow().as_ref().unwrap().scale_factor();
        }

        fn intrinsic_width(&self) -> i32 {
            return 16 * self.widget.borrow().as_ref().unwrap().scale_factor();
        }
    }
    impl SymbolicPaintableImpl for CircularProgressPaintable {
        fn snapshot_symbolic(
                &self,
                snapshot: &gdk::Snapshot,
                width: f64,
                height: f64,
                colors: &[gdk::RGBA],
        ) {
            let bounds = graphene::Rect::new(-2.0, -2.0, (width + 4.0) as f32, (width + 4.0) as f32);
            let ctx = snapshot.append_cairo(&bounds);
            let arc_end = self.progress.get() * PI * 2.0 - PI / 2.0;

            ctx.translate(width / 2.0, height / 2.0);

            let color = colors[self.color as usize];
            ctx.set_source_rgba(color.red() as f64, color.green() as f64, color.blue() as f64, color.alpha() as f64);
            
            ctx.arc(0.0, 0.0, width / 2.0 + 1.0, -PI / 2.0, arc_end);
            ctx.stroke().expect("fail to stroke");

            let mut rgba = color.clone();
            rgba.set_alpha(rgba.alpha() * 0.25);

            ctx.set_source_rgba(rgba.red() as f64, rgba.green() as f64, rgba.blue() as f64, rgba.alpha() as f64);
            ctx.arc(0.0, 0.0, width / 2.0 + 1.0, arc_end, 3.0 * PI / 2.0);
            ctx.stroke().expect("fail to stroke");
        }
    }

    impl CircularProgressPaintable {
        pub fn set_progress(&self, value: f64) {
            self.progress.set(value.clamp(0.0, 1.0));
            self.obj().invalidate_contents();
        }

        pub fn set_widget(&self, widget: Option<gtk4::Image>) {
            let obj = self.obj().clone();
            self.widget.replace(widget.clone());
            widget.unwrap().connect_notify_local(Some("scale-factor"), move |_, _| {
                obj.invalidate_size();
            });
        }
    }
}

glib::wrapper! {
    pub struct CircularProgressPaintable(ObjectSubclass<imp::CircularProgressPaintable>)
        @implements gdk::Paintable, gtk4::SymbolicPaintable;
}