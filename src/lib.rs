#![feature(trait_alias)]

use std::env;
use std::fs::read_dir;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub mod project;
pub mod ui;
pub mod protocol;

pub use project::Project;

type Projects = Box<dyn Iterator<Item = Project>>;

pub struct Config {
    sessions_folder: PathBuf,
    editor: String,
    ui: Box<dyn ui::UiFactory>,
}

impl Config {
    pub fn new() -> Config {
        let home = env::var("HOME").unwrap_or(String::from("."));
        let mut path = PathBuf::from(home);
        let mut editor = env::var("EDITOR").unwrap_or(String::from("vim"));
        let ui_name = env::var("UI");
        let ui = match ui_name {
            Ok(ui_name) => ui::from(&ui_name).expect(&format!("No '{ui_name}' UI available")),
            Err(_) => ui::new(),
        };

        if let Some(e) = ui.preferred_editor() {
            editor = e;
        }

        path.push(".local/share/nvim/sessions");

        Config {
            sessions_folder: path,
            editor,
            ui,
        }
    }

    pub fn set_folder(&mut self, path: &str) {
        self.sessions_folder = PathBuf::from(path);
    }

    pub fn create_project(&self, name: &str) -> Project {
        let mut concrete = self.sessions_folder.clone();
        concrete.push(name);
        Project::from_session_file(concrete)
    }

    pub fn get_projects(&self) -> io::Result<Projects> {
        let it = read_dir(&self.sessions_folder)?;
        let it = it.filter_map(|path| -> Option<Project> {
            if let Ok(path) = path {
                return Some(Project::from_session_file(path.path()));
            }
            None
        });

        Ok(Box::new(it))
    }

    pub fn run_ui(&self, mut projects: Projects) -> io::Result<()> {
        self.ui
            .new(projects.as_mut())
            .run()
            .map_or(Ok(()), |proj| Err(self.exec(&proj)))
    }

    pub fn exec_from(&self, mut projects: Projects, proj_name: &str) -> io::Result<()> {
        let proj = projects.find(|proj| proj.name == proj_name);
        let proj = if let Some(proj) = proj {
            proj
        } else {
            Project::from_path(proj_name)
        };
        Err(self.exec(&proj))
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
