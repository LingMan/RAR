#[macro_use] extern crate failure;
#[macro_use] extern crate nom;
#[macro_use] extern crate lazy_static;
extern crate chrono;

mod util;
mod vint;
mod signature;
mod header;
mod archive;
mod file;
mod extra;
mod end;
mod encryption;
mod extractor;

use std::io::Read;
use failure::Error;

/// The rar archive representation
#[derive(PartialEq, Debug)]
pub struct Archive {
    pub version: signature::RarSignature,
    pub details: archive::ArchiveBlock,
    pub files: Vec<file::File>,
    pub quick_open: Option<file::File>,
    pub end: end::EndBlock
}

impl Archive {
    /// Opens an .rar file and tries to parse it's content.
    /// This function returns an Archive with all the detailed information
    /// about the .rar file.
    pub fn open<R: Read>(reader: &mut R) -> Result<Archive, Error> {
        Archive::handle(reader, ExtractionOption::ExtractNone, "")
    }

    /// Extract all files of the .rar archive
    pub fn extract_all<R: Read>(reader: &mut R, path: &str) -> Result<Archive, Error> {
        Archive::handle(reader, ExtractionOption::ExtractAll, path)
    }

    /// Function to handle the .rar file in detail.
    /// Most of the other functions available are 
    /// easy to use abstraction of this function.
    pub fn handle<R: Read>(reader: &mut R, ext: ExtractionOption, path: &str) -> Result<Archive, Error> {
        // initilize the buffer
        let mut buffer = vec!();
        reader.read_to_end(&mut buffer)?;
        
        // try to parse the signature
        let (input, version) = signature::RarSignature::parse(&buffer).map_err(|_| format_err!("Can't read RAR signature"))?;
    
        // try to parse the archive information
        let (mut input, details) = archive::archive(input).map_err(|_| format_err!("Can't read RAR archive block"))?;

        let mut files = vec!();
        let mut quick_open = None;
        // loop over the packages and define how to handle them
        loop {
            // Check if it is a file
            match file::file(input) {
                Ok((i, f)) => {
                    // quick open file?
                    if f.name == "QO" {
                        input = &i[(f.head.data_area_size as usize)..];
                        quick_open = Some(f);
                        break;
                    }

                    // extract the file?
                    if ext == ExtractionOption::ExtractAll || ext == ExtractionOption::ExtractFile(f.name.clone()) {
                        extractor::extract(f.clone(), path, &i[(.. f.head.data_area_size as usize)])?;
                    }

                    // push the curser foreward and the file to the array
                    input = &i[(f.head.data_area_size as usize)..];
                    files.push(f);
                },
                Err(_) => {
                    break;
                }
            }
        }
        
        // Get the end block
        let (_, end) = end::end_block(input).map_err(|_| format_err!("Can't read RAR end"))?;

        Ok(Archive {
            version,
            details,
            files,
            quick_open,
            end
        })
    }
}

/// The different extraction options for the .rar file
#[derive(PartialEq, Debug)]
pub enum ExtractionOption {
    ExtractNone,
    ExtractAll,
    ExtractFile(String)
}

#[cfg(test)]
mod tests {
    use std::fs::{File, remove_dir_all};
    use std::io::Read;
    use ::Archive;
    use ::signature;

    // Small helper function to read a file
    fn read_file(path: &str) -> Vec<u8> {
        let mut data = vec!();
        let mut file = File::open(path).unwrap();
        file.read_to_end(&mut data).unwrap();
        data
    }

    // Get the photo globally so that every test can compare it
    lazy_static! {
        static ref PHOTO: Vec<u8> = {
            read_file("assets/photo.jpg")
        };
    }

    // Get the photo globally so that every test can compare it
    lazy_static! {
        static ref TEXT: Vec<u8> = {
            read_file("assets/text.txt")
        };
    }



    #[test]
    fn test_rar5_save_32mb_txt() {
        let rar = "rar5-save-32mb-txt";

        let mut file = File::open(format!("assets/{}.rar", rar)).unwrap();
        let archive = Archive::extract_all(&mut file, &format!("target/rar-test/{}/", rar)).unwrap();
        
        assert_eq!(archive.version, signature::RarSignature::RAR5);
        assert_eq!(archive.files[0].name, "text.txt");
        assert_eq!(archive.files[0].unpacked_size, 2118);
        assert_eq!(*TEXT, read_file(&format!("target/rar-test/{}/text.txt", rar)));

        remove_dir_all(&format!("target/rar-test/{}", rar)).unwrap();
    }

    #[test]
    fn test_rar5_save_32mb_txt_png() {
        let mut file = File::open("assets/rar5-save-32mb-txt-png.rar").unwrap();
        let archive = Archive::extract_all(&mut file, "target/rar-test/rar5-save-32mb-txt-png/").unwrap();

        assert_eq!(archive.version, signature::RarSignature::RAR5);
        assert_eq!(archive.files[0].name, "photo.jpg");
        assert_eq!(archive.files[0].unpacked_size, 2149083);
        assert_eq!(archive.files[1].name, "text.txt");
        assert_eq!(archive.files[1].unpacked_size, 2118);
        assert_eq!(archive.quick_open.unwrap().name, "QO");
        assert_eq!(*TEXT, read_file("target/rar-test/rar5-save-32mb-txt-png/text.txt"));
        assert_eq!(*PHOTO, read_file("target/rar-test/rar5-save-32mb-txt-png/photo.jpg"));

        remove_dir_all("target/rar-test/rar5-save-32mb-txt-png/").unwrap();
    }

    #[test]
    #[ignore]
    fn test_rar5_save_32mb_txt_png_pw_test() {
        let mut file = File::open("assets/rar5-save-32mb-txt-png-pw-test.rar").unwrap();
        let archive = Archive::extract_all(&mut file, "target/rar-test/rar5-save-32mb-txt-png-pw-test/").unwrap();

        assert_eq!(archive.version, signature::RarSignature::RAR5);
        assert_eq!(archive.files[0].name, "photo.jpg");
        assert_eq!(archive.files[0].unpacked_size, 2149083);
        assert_eq!(archive.files[1].name, "text.txt");
        assert_eq!(archive.files[1].unpacked_size, 2118);
        assert_eq!(archive.quick_open.unwrap().name, "QO");
        assert_eq!(*TEXT, read_file("target/rar-test/rar5-save-32mb-txt-png-pw-test/text.txt"));
        assert_eq!(*PHOTO, read_file("target/rar-test/rar5-save-32mb-txt-png-pw-test/photo.jpg"));

        remove_dir_all("target/rar-test/rar5-save-32mb-txt-png-pw-test/").unwrap();
    }
}
