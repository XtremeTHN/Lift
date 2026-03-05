use std::{cell::RefCell, fs::File, path::{Path, PathBuf}};

use gtk4::{
    CompositeTemplate, ListBoxRow,
    gdk::Texture,
    gio::{self, prelude::FileExt},
    glib::{self, Object, object::Cast, variant::ToVariant},
    prelude::WidgetExt,
    subclass::prelude::*,
};

use glib::subclass::InitializingObject;
use log::info;

use crate::roms::{
    formats::{
        nacp::Nacp, nca::{ContentType, Nca}, xci::Xci
    }, fs::{
        pfs::{PFSHeader, PartitionFs, PartitionFsHeader},
        romfs::RomFs,
    }, keyring::Keyring, readers::{EncryptedCtrFileRegion, FileRegion}
};

mod imp {
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/rom.ui")]
    pub struct Rom {
        #[template_child]
        pub frame: TemplateChild<gtk4::Frame>,

        #[template_child]
        pub icon: TemplateChild<gtk4::Picture>,

        #[template_child]
        pub rom_title: TemplateChild<gtk4::Label>,

        #[template_child]
        pub rom_version: TemplateChild<gtk4::Label>,

        #[template_child]
        pub rom_size: TemplateChild<gtk4::Label>,

        #[template_child]
        pub end_button_image: TemplateChild<gtk4::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Rom {
        const NAME: &'static str = "Rom";

        type Type = super::Rom;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Rom {}
    impl WidgetImpl for Rom {}
    impl ListBoxRowImpl for Rom {}
}

glib::wrapper! {
    pub struct Rom(ObjectSubclass<imp::Rom>)
        @extends gtk4::Widget, ListBoxRow,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

use binrw::BinRead;
use std::io::{Read, Seek};
use positioned_io::ReadAt;


#[gtk4::template_callbacks]
impl Rom {
    async fn fallback(&self, file: String) {
        let f = gio::File::for_path(file);
        let i = f
            .query_info_future(
                "standard::size,standard::name",
                gio::FileQueryInfoFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await;
        let obj = self.imp();

        match i {
            Ok(info) => {
                let name = info.name();
                let ext = name.extension();
                let size =
                    glib::format_size_full(info.size() as u64, glib::FormatSizeFlags::DEFAULT);

                if ext.is_none() {
                    obj.rom_version.set_label("Format: unknown");
                } else {
                    obj.rom_version.set_label(&format!(
                        "Format: {}",
                        ext.unwrap().to_string_lossy().to_string()
                    ));
                }

                obj.rom_title.set_label(&name.to_string_lossy().to_string());
                obj.rom_size.set_label(&format!("Size: {}", size));
            }

            Err(err) => {
                self.activate_action("win.toast", Some(&err.to_string().to_variant()))
                    .expect("couldn't send action");
            }
        }
    }

    fn send_error(&self, message: &str) {
        self.activate_action("win.toast", Some(&message.to_string().to_variant())).expect("toast");
    }

    pub fn new() -> Self {
        Object::builder().build()
    }

    fn set_texture_from_bytes(&self, data: Vec<u8>) {
        let bytes = glib::Bytes::from(&data);
        let txt = Texture::from_bytes(&bytes);
        
        if let Err(e) = &txt {
            self.send_error(&format!("Couldn't get texture: {}", e.to_string()))
        }

        self.imp().icon.set_paintable(Some(&txt.unwrap()));
    }

    fn set_from_nacp(&self, nacp: Nacp) {
        let obj = self.imp();

        for x in &nacp.titles {
            if x.raw_name.iter().all(|&b| b == 0) {
                continue;
            }

            let name = x.name();
            if let Err(e) = &name {
                self.send_error(&format!("Couldn't decode title: {}", e.to_string()))
            }

            let unwrapped_name = name.unwrap();
            obj.rom_title.set_label(&unwrapped_name);
        }
        
        let ver = nacp.version();
        match ver {
            Ok(version) => {
                obj.rom_version.set_label(&format!("Version: {}", version));
            }
            Err(e) => {
                self.send_error(&format!("Couldn't decode rom version: {}", e.to_string()));
            }
        }
    }

    pub fn populate_from_file(&self, file: PathBuf) {
        let ext = file.extension().unwrap();

        if ext == "nsp" {
            self.handle_nsp(file.to_string_lossy().to_string());
        }
    }

    fn parse_pfs<R: ReadAt + Read + Seek>(&self, pfs: PartitionFs<PartitionFsHeader, R>, keyring: &Keyring) {
        for x in pfs.header.entry_table().iter() {
            let e = pfs.open_entry(x);

            let nca = Nca::new(&keyring, e);

            if let Err(e) = &nca {
                self.send_error(&format!("Error while parsing nca: {}", e.to_string()))
            }

            let mut unwrapped_nca = nca.unwrap();

            if unwrapped_nca.header.content_type != ContentType::Control {
                continue;
            }

            let fs = unwrapped_nca.open_fs(0);
            if let Err(e) = &fs {
                self.send_error(&format!("Error while opening fs: {}", e.to_string()))
            }

            let mut unwrapped_fs = fs.unwrap();
            let romfs = RomFs::new(&mut unwrapped_fs);
            if let Err(e) = &romfs {
                self.send_error(&format!("Error while trying to open romfs: {}", e.to_string()))
            }

            let unwrapped_romfs = romfs.unwrap();
            
            let mut applied_image = false;
            let mut applied_nacp = false;
            for x in unwrapped_romfs.files.iter() {
                let name = unwrapped_romfs.get_name_for_entry(x).expect("failed");
                let f = PathBuf::from(name);
                let ext = f.extension().unwrap();

                let mut s = unwrapped_romfs.get_file(x);
                if ext == "dat" && !applied_image {
                    let mut buf = vec![0u8; x.size as usize];
                    s.read_exact(&mut buf);
                    self.set_texture_from_bytes(buf);

                    applied_image = true;
                }

                if ext == "nacp" && !applied_nacp {
                    let nacp = Nacp::read(&mut s);
                    if let Err(e) = &nacp {
                        self.send_error(&format!("Error while obtaining rom info: {}", e.to_string()))
                    }

                    let unwrapped_nacp = nacp.unwrap();
                    self.set_from_nacp(unwrapped_nacp);

                    applied_nacp = true;
                }

                if applied_image && applied_nacp {
                    break;
                }
            }
        }
    }

    pub fn handle_xci(&self) {
        gio::spawn_blocking(|| {
            // let rom = 
        });
    }

    pub fn handle_nsp(&self, file_path: String) {
        let obj = self.clone();

        glib::MainContext::default().spawn_local(async move {
            let f = File::open(file_path.clone());
            if let Err(e) = &f {
                obj.send_error(&e.to_string())
            }

            let mut file = f.unwrap();
            let pfs = PartitionFs::new_default_header(&mut file);
            let mut keyring = Keyring::new();
            keyring.parse();

            match pfs {
                Ok(fs) => {
                    obj.parse_pfs(fs, &keyring);
                }
                Err(e) => { 
                    obj.send_error(&e.to_string())
                }
            }
        });
    }

    #[template_callback]
    fn remove_rom(&self, _: &gtk4::Button) {
        if let Some(parent) = self.parent() {
            if let Ok(listbox) = parent.downcast::<gtk4::ListBox>() {
                listbox.remove(self);
            }
        }
    }
}
