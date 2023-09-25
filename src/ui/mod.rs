use crate::Project;
use std::env;
use std::io;
use std::iter::Iterator;

pub trait Ui {
    fn run(&self) -> io::Result<()>;
}

pub fn new<Projects: Iterator<Item = Project>>(projects: Projects) -> Box<dyn Ui> {
    if env::var("GTK").is_ok() {
        Box::new(gtk::Gtk::new(projects))
    } else {
        Box::new(Stdout::new(projects))
    }
}

pub struct Stdout {
    projects: Vec<Project>,
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
    fn run(&self) -> io::Result<()> {
        println!("Please choose from one of next projects:");
        print_projects(&self.projects);
        println!("And rerun with `{} <project>`", env::args().next().unwrap());
        Ok(())
    }
}

impl Stdout {
    pub fn new<Projects: Iterator<Item = Project>>(projects: Projects) -> Stdout {
        Stdout {
            projects: projects.collect(),
        }
    }
}

mod gtk;
