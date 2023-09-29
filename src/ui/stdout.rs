use super::super::project::Project;
use super::{UiFactory, Projects, Ui};
use std::env;

struct Stdout {
    projects: Vec<Project>,
}

pub struct StdoutFactory {}

impl UiFactory for StdoutFactory {
    fn new<'a>(&self, projects: Projects<'a>) -> Box<dyn Ui> {
        Box::new(Stdout {
            projects: projects.collect(),
        })
    }
}

fn print_project(project: &Project, shift: usize) {
    println!(
        "\t{name:<shift$} at {path}",
        name = project.name,
        path = project.path.to_str().unwrap()
    )
}

fn print_projects(projects: &Vec<Project>) {
    let shift = Project::max_name_length(projects);

    for project in projects.iter() {
        print_project(&project, shift);
    }
}

impl Ui for Stdout {
    fn run(&mut self) -> Option<Project> {
        println!("Please choose from one of next projects:");
        print_projects(&self.projects);
        println!("And rerun with `{} <project>`", env::args().next().unwrap());
        None
    }
}

