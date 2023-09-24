use std::env;
use std::fs::read_dir;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub mod ui;

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
}

pub struct Config {
    sessions_folder: PathBuf,
    editor: String,
}

impl Config {
    pub fn new() -> Config {
        let home = env::var("HOME").unwrap_or(String::from("."));
        let mut path = PathBuf::from(home);
        let editor = env::var("EDITOR").unwrap_or(String::from("vim"));

        path.push(".local/share/nvim/sessions");

        Config {
            sessions_folder: path,
            editor,
        }
    }

    pub fn set_folder(&mut self, path: &str) {
        self.sessions_folder = PathBuf::from(path);
    }

    pub fn create_project(&self, name: &str) -> Project {
        let mut concrete = self.sessions_folder.clone();
        concrete.push(name);
        Project::build(concrete)
    }

    pub fn get_projects(&self) -> io::Result<Box<dyn Iterator<Item = Project>>> {
        let it = read_dir(&self.sessions_folder)?;
        let it = it.filter_map(|path| -> Option<Project> {
            if let Ok(path) = path {
                return Some(Project::build(path.path()));
            }
            None
        });

        Ok(Box::new(it))
    }

    pub fn exec(&self, project: &Project) -> io::Error {
        println!(
            "Running {} at {}",
            self.editor,
            project.path.to_str().unwrap()
        );
        Command::new(&self.editor).current_dir(&project.path).exec()
    }
}

#[cfg(test)]
mod tests {
    use crate::{widthdraw_path_from_session, widthdraw_path_from_session_name, Config};
    use std::path::PathBuf;

    #[test]
    fn check_config() {
        let mut cfg = Config::new();
        cfg.set_folder("/");
        let prj = cfg.create_project("__tmp__test1__test2");
        assert_eq!(prj.get_path(), "/tmp/test1/test2")
    }

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
