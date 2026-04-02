use std::io::Seek;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use std::{fs::File, io::Read};

use gtk::{
    gdk,
    gio::{self, prelude::FileExt},
    glib,
};
use nxroms::formats::cnmt::PackagedContentMetaHeader;
use nxroms::formats::nacp::{Nacp, Title, TitleLanguage};
use nxroms::formats::nca::{ContentType, Nca, NcaErrors};
use nxroms::formats::xci::{Xci, XciErrors};
use nxroms::fs::pfs::PartitionFsErrors;
use nxroms::fs::romfs::{RomFs, RomFsErrors, RomFsFileEntry};
use nxroms::keyring::{Keyring, KeyringErrors};
use nxroms::{
    formats::cnmt::ContentMetaType,
    fs::pfs::{PFSHeader, PartitionFs},
    // readers::
};

use binrw::BinRead;

#[derive(Debug)]
pub struct RomData {
    pub texture_data: Option<gdk::Texture>,
    pub title: String,
    pub version: String,
    pub meta_type: ContentMetaType,
    pub size: i64,
    pub error: Option<HandlingErrors>,
}

#[derive(thiserror::Error, Debug)]
pub enum FromGFileErrors {
    #[error("File object has no path")]
    NoPath,
    #[error("File has no extension: {0}")]
    NoExtension(PathBuf),
    #[error("File is not a rom: {0}")]
    InvalidExt(PathBuf),
    #[error("GLib error: {0}")]
    GLib(#[from] glib::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum NacpErrors {
    #[error("Decoding error: {0}")]
    Decoding(#[from] FromUtf8Error),
    #[error("No suitable language")]
    NoSuitableLanguage,
}

#[derive(thiserror::Error, Debug)]
pub enum HandlingErrors {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Couldn't parse kerying: {0}")]
    CorruptKeyring(#[from] KeyringErrors),
    #[error("Couldn't parse rom: {0}")]
    CorruptRom(#[from] binrw::Error),
    #[error("Couldn't parse xci: {0}")]
    CorruptXci(#[from] XciErrors),
    #[error("PFS Error")]
    CorruptPfs(#[from] PartitionFsErrors),
    #[error("Couldn't parse romfs: {0}")]
    CorruptRomFs(#[from] RomFsErrors),
    #[error("Couldn't parse nca: {0}")]
    CorruptNca(#[from] NcaErrors),
    #[error("Couldn't parse nacp: {0}")]
    CorruptNacp(#[from] NacpErrors),
    #[error("Couldn't build texture: {0}")]
    CorruptTexture(#[from] glib::Error),
    #[error("Couldn't find nacp in romfs")]
    NoNacp,
    #[error("Couldn't find cnmt in rom")]
    NoCnmt,
}

fn matches_extension(file: String, extension: &str) -> bool {
    let parts = file.split(".").collect::<Vec<&str>>();
    let ext = parts.last();

    ext == Some(&extension)
}

#[derive(Debug)]
pub enum RomType {
    Xci,
    Nsp,
}

pub struct RomDataLoader {
    path: PathBuf,
    ext: String,
    language: TitleLanguage,
    keyring_path: String,
    size: i64,
}

impl RomDataLoader {
    fn check_title(&self, title: &Title) -> bool {
        !title.raw_name.iter().all(|v| v == &0)
    }

    fn handle_nacp(&self, nacp: Nacp) -> Result<(String, String), NacpErrors> {
        let title = nacp
            .titles
            .get(self.language as usize)
            .filter(|t| self.check_title(t))
            .or_else(|| nacp.titles.first())
            .ok_or(NacpErrors::NoSuitableLanguage)?;

        if title.raw_name.iter().all(|v| v == &0) {}

        Ok((title.name()?, nacp.version()?))
    }

    fn read_icon<S: Read + Seek + positioned_io::ReadAt>(
        &self,
        romfs: &RomFs,
        romfs_stream: &mut S,
        entry: &RomFsFileEntry,
    ) -> glib::Bytes {
        let mut icon_stream = romfs.open_file(entry, romfs_stream);
        let mut icon_buffer = vec![0u8; entry.size as usize];

        icon_stream.read_exact(&mut icon_buffer);

        glib::Bytes::from(&icon_buffer)
    }

    fn handle_control_nca<S: Read + Seek + positioned_io::ReadAt>(
        &self,
        mut nca: Nca,
        stream: &mut S,
    ) -> Result<((String, String), gdk::Texture), HandlingErrors> {
        let mut romfs_stream = nca.open_fs(0, stream)?;

        let romfs = RomFs::new(&mut romfs_stream)?;

        let mut res: Option<(String, String)> = None;
        let mut icon: Option<gdk::Texture> = None;
        let mut icon_entries: Vec<&RomFsFileEntry> = vec![];

        for (index, entry) in romfs.files.iter().enumerate() {
            match romfs.get_name_for_entry(entry) {
                Ok(name) => {
                    if matches_extension(name.clone(), "nacp") {
                        let mut nacp_stream = romfs.open_file(entry, &mut romfs_stream);
                        let nacp = Nacp::read(&mut nacp_stream)?;

                        res = Some(self.handle_nacp(nacp)?);
                    } else if matches_extension(name.clone(), "dat") {
                        icon_entries.push(entry);

                        // try to find the icon corresponding to the language set
                        // in the constructor.
                        // the file is named smth like this: icon_AmericanEnglish.dat
                        // so just remove the .dat and split by "_"
                        let trimmed = name.replace(".dat", "");
                        let parts = trimmed.split("_").collect::<Vec<&str>>();

                        // then just compare the lang part to the choosed language
                        if icon.is_some() {
                            continue;
                        }

                        if let Some(lang) = parts.last()
                            && &self.language.to_string() == lang
                        {
                            let bytes = self.read_icon(&romfs, &mut romfs_stream, entry);
                            icon = Some(gdk::Texture::from_bytes(&bytes)?);
                        }
                    }
                }
                Err(e) => log::warn!(
                    "RomDataLoader::handle_control_nca(): couldn't decode name for file in index {}: {}",
                    index,
                    e
                ),
            }
        }

        if res.is_none() {
            return Err(HandlingErrors::NoNacp);
        }

        if icon.is_none() && !icon_entries.is_empty() {
            if let Some(first_icon) = icon_entries.first() {
                let bytes = self.read_icon(&romfs, &mut romfs_stream, first_icon);
                icon = Some(gdk::Texture::from_bytes(&bytes)?);
            }
        }

        Ok((res.unwrap(), icon.unwrap()))
    }

    fn handle_meta_nca<S: Read + Seek + positioned_io::ReadAt>(
        &self,
        mut nca: Nca,
        stream: &mut S,
    ) -> Result<ContentMetaType, HandlingErrors> {
        let mut fs = nca.open_fs(0, stream)?;
        let pfs = PartitionFs::new_pfs0_header(&mut fs)?;

        for entry in pfs.header.entry_table.iter() {
            let name = pfs.get_name_for_entry(entry).expect("couldn't get");
            let parts = name.split(".").collect::<Vec<&str>>();
            if parts.last() != Some(&"cnmt") {
                continue;
            }

            let header = PackagedContentMetaHeader::read(&mut fs)?;
            return Ok(header.content_meta_type);
        }

        Err(HandlingErrors::NoCnmt)
    }

    fn find_and_handle_info<H, S>(
        &self,
        pfs: PartitionFs<H>,
        stream: &mut S,
    ) -> Result<RomData, HandlingErrors>
    where
        H: binrw::BinRead + PFSHeader,
        S: Read + Seek + positioned_io::ReadAt,
    {
        let mut keyring = Keyring::new(self.keyring_path.clone());
        keyring.parse()?;

        // defaults
        let mut name_and_version: (String, String) = (
            self.path.file_name().unwrap().to_string_lossy().to_string(),
            String::from("0.0.0"),
        );
        let mut cnmt_type: Option<ContentMetaType> = None;
        let mut texture: Option<gdk::Texture> = None;

        for entry in pfs.header.entry_table() {
            let name = pfs.get_name_for_entry(entry)?;

            if !matches_extension(name, "nca") {
                continue;
            }

            let mut entry_fs = pfs.open_entry(entry, &mut *stream);
            let nca = Nca::new(&keyring, &mut entry_fs)?;

            match nca.header.content_type {
                ContentType::Control => {
                    let res = self.handle_control_nca(nca, &mut entry_fs)?;
                    name_and_version = res.0;
                    texture = Some(res.1);
                }
                ContentType::Meta => {
                    cnmt_type = Some(self.handle_meta_nca(nca, &mut entry_fs)?);
                }
                _ => {}
            }
        }

        // FIXME: if this crashes, handle

        let data = RomData {
            texture_data: texture,
            title: name_and_version.0,
            version: name_and_version.1,
            meta_type: cnmt_type.unwrap(),
            size: self.size,
            error: None,
        };

        Ok(data)
    }

    fn handle_nsp(&self) -> Result<RomData, HandlingErrors> {
        let mut file = File::open(self.path.clone())?;

        let pfs = PartitionFs::new_pfs0_header(&mut file)?;
        Ok(self.find_and_handle_info(pfs, &mut file)?)
    }

    fn handle_xci(&self) -> Result<RomData, HandlingErrors> {
        let mut file = File::open(self.path.clone())?;

        let mut xci = Xci::new(&mut file)?;
        let mut partition = xci.open_partition("secure".to_string(), &mut file)?;
        let pfs = xci.open_partition_fs(&mut partition)?;

        Ok(self.find_and_handle_info(pfs, &mut partition)?)
    }

    pub fn from_gfile(
        file: gio::File,
        language: TitleLanguage,
        keyring_path: String,
    ) -> Result<Self, FromGFileErrors> {
        let path = file.path().ok_or(FromGFileErrors::NoPath)?;

        let _path = path.clone();
        let ext = _path
            .extension()
            .ok_or(FromGFileErrors::NoExtension(path.clone()))?
            .to_string_lossy()
            .to_string();

        if ext != "nsp" && ext != "xci" {
            return Err(FromGFileErrors::InvalidExt(_path));
        }

        let querier = file.query_info(
            "standard::size",
            gio::FileQueryInfoFlags::NONE,
            None::<&gio::Cancellable>,
        )?;

        Ok(Self {
            keyring_path,
            ext,
            path,
            language,
            size: querier.size(),
        })
    }

    pub fn load_default(&self, error: HandlingErrors) -> RomData {
        let name = self.path.file_name().unwrap().to_string_lossy().to_string();

        RomData {
            texture_data: None,
            title: name,
            version: String::from("0.0.0"),
            meta_type: ContentMetaType::Application,
            size: self.size,
            error: Some(error),
        }
    }

    pub fn load(&self) -> Result<RomData, HandlingErrors> {
        if self.ext == "nsp" {
            return self.handle_nsp();
        } else {
            return self.handle_xci();
        }
    }
}
