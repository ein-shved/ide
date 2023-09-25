use std::env;
use std::fs::read_dir;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub mod ui;
pub mod project;

use project::Project as Project;

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
    use super::Config;

    #[test]
    fn check_config() {
        let mut cfg = Config::new();
        cfg.set_folder("/");
        let prj = cfg.create_project("__tmp__test1__test2");
        assert_eq!(prj.get_path(), "/tmp/test1/test2")
    }
}
