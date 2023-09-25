use paste;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub session_file: PathBuf,
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
    pub fn new(session_file: &str) -> Project {
        Self::build(PathBuf::from(session_file))
    }

    pub fn build(session_file: PathBuf) -> Project {
        let path = widthdraw_path_from_session(&session_file);
        Project {
            name: String::from(
                path.file_name()
                    .expect("Invalid project session_file {session_file}")
                    .to_str()
                    .unwrap(),
            ),
            path,
            session_file,
        }
    }

    pub fn get_path(&self) -> &str {
        &self.path.to_str().unwrap_or("")
    }

    max_length!(name);
    max_length!(path);
    max_length!(session_file);
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
