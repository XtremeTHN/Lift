use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
    string::FromUtf8Error,
};

use binrw::BinRead;
use gtk4::{
    gio::{Settings, prelude::SettingsExt},
    glib,
};
use positioned_io::ReadAt;

use nxroms::{
    formats::{
        nacp::{Nacp, TitleLanguage},
        nca::{ContentType, Nca, NcaErrors},
        cnmt::{PackagedContentMetaHeader, ContentMetaType},
        xci::{Xci, XciErrors},
    },
    fs::{
        pfs::{PFSHeader, PartitionFs, PartitionFsErrors},
        romfs::{RomFs, RomFsErrors},
    },
    keyring::{Keyring, KeyringErrors},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PopulateError {
    #[error("File {0} does not have an extension")]
    NoExtension(String),
    #[error("File {0} has an invalid extension")]
    NotARom(String),
    #[error("Handle error: {0}")]
    HandleErr(#[from] HandleError),
}

#[derive(Error, Debug)]
pub enum FindInfoFilesError {
    #[error("Couldn't construct image_data: {0}")]
    GLibError(#[from] glib::Error),
    #[error("Couldn't decode name: {0}")]
    DecodingError(#[from] FromUtf8Error),
    #[error("Error while parsing keyring: {0}")]
    KeyringParse(#[from] KeyringErrors),
    #[error("File in romfs has no extension: {0}")]
    NoExtension(String),
    #[error("Couldn't parse nca: {0}")]
    NcaError(#[from] NcaErrors),
    #[error("Couldn't construct Nacp: {0}")]
    NacpError(#[from] binrw::Error),
    #[error("Couldn't read romfs: {0}")]
    RomFS(#[from] RomFsErrors),
    #[error("Error while reading entry: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Couldn't find nacp file")]
    NacpNotFound,
}

#[derive(Error, Debug)]
pub enum HandleError {
    #[error("Error while parsing xci: {0}")]
    Xci(#[from] XciErrors),
    #[error("Error while trying to get PFS: {0}")]
    Pfs(#[from] PartitionFsErrors),
    #[error("Error while trying to read stream: {0}")]
    Read(#[from] binrw::Error),
    #[error("Error while parsing keyring: {0}")]
    KeyringParse(#[from] KeyringErrors),
    #[error("Error while trying to find info files: {0}")]
    Find(#[from] FindInfoFilesError),
    #[error("Error while decoding nacp fields: {0}")]
    Decoding(#[from] FromUtf8Error),
}

pub struct RomInfo {
    pub title: Option<String>,
    pub version: Option<String>,
    pub image_data: Option<Vec<u8>>,
    pub language: TitleLanguage,
    pub meta_type: Option<ContentMetaType>,
    file: File,
    path: PathBuf,
}

impl RomInfo {
    pub fn new(path: PathBuf, language: TitleLanguage) -> std::io::Result<Self> {
        let file = File::open(&path)?;

        Ok(Self {
            title: None,
            version: None,
            image_data: None,
            language,
            meta_type: None,
            file,
            path,
        })
    }

    fn handle_nacp(&mut self, nacp: Nacp) -> Result<(), FromUtf8Error> {
        if let Some(title) = nacp.titles.get(self.language as usize) {
            self.title = Some(title.name()?);
        } else {
            log::warn!("No title name");
        }

        self.version = Some(nacp.version()?);

        Ok(())
    }

    fn get_keyring(&self) -> Result<Keyring, KeyringErrors> {
        let settings = Settings::new("com.github.XtremeTHN.Lift");
        let path = settings.string("keys-path");
        let mut keys = Keyring::new(path.to_string());
        keys.parse()?;

        Ok(keys)
    }

    fn find_info_files<T: BinRead + PFSHeader, R: ReadAt + Read + Seek>(
        &self,
        pfs: PartitionFs<T>,
        part: R,
    ) -> Result<(Nacp, Option<Vec<u8>>, PackagedContentMetaHeader), FindInfoFilesError> {
        let keyring = self.get_keyring()?;

        let mut meta_header: Option<PackagedContentMetaHeader> = None;
        let mut nacp: Option<Nacp> = None;
        let mut texture: Option<Vec<u8>> = None;
        for (index, entry) in pfs.header.entry_table().iter().enumerate() {
            let name = pfs.get_name_for_entry(entry).expect("failed to get name:");

            let mut r = pfs.open_entry(entry, &part);

            if name.split(".").collect::<Vec<&str>>()[1] != "nca" {
                continue;
            }

            let mut nca = Nca::new(&keyring, &mut r).expect("err");
            
            match nca.header.content_type {
                ContentType::Control => {
                    let mut fs = nca.open_fs(0, &mut r)?;
                    let rom_fs = RomFs::new(&mut fs)?;
                    for x in rom_fs.files.iter() {
                        let name = String::from_utf8(x.name.clone())?;
                        let unwrapped = PathBuf::from(&name);
                        let ext = unwrapped.extension();
                        if ext.is_none() {
                            return Err(FindInfoFilesError::NoExtension(name));
                        }

                        let ext_unwrapped = ext.unwrap();
                        if ext_unwrapped == "dat" && texture.is_none() {
                            let mut buf = vec![0u8; x.size as usize];
                            let mut reg = rom_fs.open_file(x, &mut fs);

                            reg.read_exact(&mut buf)?;

                            texture = Some(buf);
                        }

                        if ext_unwrapped == "nacp" && nacp.is_none() {
                            let mut reg = rom_fs.open_file(x, &mut fs);
                            nacp = Some(Nacp::read(&mut reg).expect("asd"));
                        }
                    } 
                }
                ContentType::Meta => {
                    let mut fs = nca.open_fs(0, &mut r)?;
                    let header = PackagedContentMetaHeader::read(&mut fs)?;
                    meta_header = Some(header);
                }
                _ => {}
            }
            
            if nacp.is_some() && meta_header.is_some() {
                break;
            }
        }

        if nacp.is_none() {
            return Err(FindInfoFilesError::NacpNotFound);
        }

        Ok((nacp.unwrap(), texture, meta_header.unwrap()))
    }

    fn handle_xci(&mut self) -> Result<(), HandleError> {
        let mut xci = Xci::new(&mut self.file)?;

        let mut part = xci.open_partition("secure".to_string(), &self.file)?;
        let pfs = xci.open_partition_fs(&mut part)?;
        let (nacp, texture, meta_header) = self.find_info_files(pfs, &mut part)?;

        self.image_data = texture;
        self.handle_nacp(nacp)?;
        self.meta_type = Some(meta_header.content_meta_type);
        
        Ok(())
    }

    fn handle_nsp(&mut self) -> Result<(), HandleError> {
        let pfs = PartitionFs::new_pfs0_header(&mut self.file)?;
        let (nacp, texture, meta_header) = self.find_info_files(pfs, &self.file)?;

        self.image_data = texture;
        self.handle_nacp(nacp)?;
        self.meta_type = Some(meta_header.content_meta_type);
        Ok(())
    }

    pub fn populate(&mut self) -> Result<(), PopulateError> {
        let extension = self.path.extension();
        let path_str = self.path.to_string_lossy().to_string();
        if extension.is_none() {
            return Err(PopulateError::NoExtension(path_str));
        }

        match extension.unwrap().to_str().unwrap() {
            "nsp" => Ok(self.handle_nsp()?),

            "xci" => Ok(self.handle_xci()?),

            _ => Err(PopulateError::NotARom(path_str)),
        }
    }
}
