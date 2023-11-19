use paste;
use std::path::PathBuf;
use std::fs;
use std::io;

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub session_file: Option<PathBuf>,
    pub exists: bool,
}

fn widthdraw_path_from_session_name(path: &str) -> PathBuf {
    let mut res = PathBuf::from("/");
    res.push(path.split("__").collect::<PathBuf>());
    res
}

fn widthdraw_path_from_session(path: &PathBuf) -> PathBuf {
    widthdraw_path_from_session_name(
        path.file_name()
            .expect("Invalid session path")
            .to_str()
            .unwrap(),
    )
}

trait Len {
    fn len(&self) -> usize;
}

impl Len for PathBuf {
    fn len(&self) -> usize
    {
        self.to_str().unwrap().len()
    }
}

macro_rules! max_length {
    ( $field:ident) => {
        paste::item! {
            pub fn [< max_ $field _length >]
                <'a, Cont: IntoIterator<Item = &'a Project>>
                    (items: Cont) -> usize {
                let mut max_len = 0;
                for proj in items {
                    if max_len < proj.$field.len() {
                        max_len = proj.$field.len()
                    }
                }
                max_len
            }
        }
    };
}

impl Project {
    pub fn from_session_file(session_file: PathBuf) -> Project {
        let path = widthdraw_path_from_session(&session_file);
        Self::build(path, Some(session_file))
    }

    pub fn from_path(path: &str) -> Project {
        let path = PathBuf::from(path);
        Self::build(path, None)
    }

    fn build(path: PathBuf, session_file: Option<PathBuf>) -> Project {
        let mut exists = true;
        let path = match fs::canonicalize(path.clone()) {
            Ok(path) => path,
            Err(what) => {
                println!("Wrong project path detected at '{}': {}",path.to_str().unwrap(), what );
                exists = false;
                path
            },
        };
        Project {
            name: String::from(
                path.file_name()
                    .expect(&format!("Invalid project path '{}'", path.to_str().unwrap()))
                    .to_str()
                    .unwrap(),
            ),
            path,
            session_file,
            exists
        }
    }

    pub fn rm(&self) -> io::Result<()> {
        if let Some(session_file) = &self.session_file {
            fs::remove_file(session_file)
        } else {
            Ok(())
        }
    }

    pub fn get_path(&self) -> &str {
        &self.path.to_str().unwrap_or("")
    }

    max_length!(name);
    max_length!(path);
}

#[cfg(test)]
mod tests {
    use super::{widthdraw_path_from_session, widthdraw_path_from_session_name};
    use std::path::PathBuf;

    #[test]
    fn check_widthdraw_path_from_session_name() {
        assert_eq!(
            widthdraw_path_from_session_name("__tmp__test1__test2"),
            PathBuf::from("/tmp/test1/test2")
        )
    }

    #[test]
    fn check_widthdraw_path_from_session() {
        assert_eq!(
            widthdraw_path_from_session(&PathBuf::from("~/.local//sessions/__tmp__test1__test2")),
            PathBuf::from("/tmp/test1/test2")
        )
    }
}
