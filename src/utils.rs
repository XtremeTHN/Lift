use gtk::{
    gio::{
        self,
        prelude::{FileExt, ListModelExt},
    },
    glib::{self, object::IsA, variant::ToVariant},
    prelude::WidgetExt,
};

use adw::prelude::Cast;
use core::future::Future;

pub fn send_error<W: IsA<gtk::Widget>>(widget: &W, message: &str) {
    log::error!("{}: {}", widget.widget_name(), message);
    widget
        .activate_action("win.toast", Some(&message.to_string().to_variant()))
        .expect("toast");
}

pub async fn iterate_model_async<Fut, F>(model: gio::ListModel, mut func: F)
where
    Fut: Future<Output = bool>,
    F: FnMut(gio::File, u32) -> Fut,
{
    for x in 0..model.n_items() {
        if let Some(obj) = model.item(x) {
            let f = obj.downcast::<gio::File>();
            match f {
                Ok(file) => {
                    if !func(file, x).await {
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

#[derive(Default, Debug)]
pub struct CancellableAsyncTasks<R: 'static> {
    tasks: Vec<glib::JoinHandle<R>>,
}

impl<R: 'static> CancellableAsyncTasks<R> {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn spawn_task<F: Future<Output = R> + 'static>(&mut self, task: F) {
        self.tasks.push(glib::spawn_future_local(task));
    }

    pub fn cancel_all(&mut self) {
        for task in self.tasks.iter() {
            task.abort();
        }

        self.tasks.clear();
    }
}

pub struct FileVecBuilder {
    pub files: Vec<String>,
    prefix: Option<String>,
    suffix: Option<String>,
}

impl FileVecBuilder {
    pub fn new() -> Self {
        Self {
            files: vec![],
            prefix: None,
            suffix: None,
        }
    }

    pub fn gfiles(mut self, files: Vec<gio::File>) -> Self {
        for x in files.iter() {
            if let Some(p) = x.path() {
                self = self.file(p.to_string_lossy().to_string());
            };
        }

        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn suffix(mut self, suffix: &str) -> Self {
        self.suffix = Some(suffix.to_string());
        self
    }

    pub fn file(mut self, file: String) -> Self {
        let formatted = format!(
            "{}{}{}\n",
            self.prefix.as_deref().unwrap_or(""),
            file,
            self.suffix.as_deref().unwrap_or("")
        );
        self.files.push(formatted);

        self
    }

    pub fn build(self) -> Vec<Vec<u8>> {
        self.files.iter().map(|s| s.bytes().collect()).collect()
    }

    pub fn flatten_build(self) -> Vec<u8> {
        self.files.iter().flat_map(|s| s.bytes()).collect()
    }

    pub fn build_net(self) -> Vec<u8> {
        let e = self.flatten_build();
        let length = e.len() as u32;
        let mut buf = length.to_be_bytes().to_vec();
        buf.extend_from_slice(&e);

        buf
    }
}
