use crate::Project;
use std::env;
use std::iter::Iterator;

mod gtk;
mod stdout;

pub trait Ui {
    fn run(&self) -> Option<Project>;
}

type Projects<'a> = &'a mut dyn Iterator<Item = Project>;

pub trait UiFactory {
    fn new<'a>(&self, projects: Projects<'a>) -> Box<dyn Ui>;
}

pub fn new() -> Box<dyn UiFactory> {
    Box::new(gtk::GtkFactory {})
}

pub fn from(name: &str) -> Option<Box<dyn UiFactory>> {
    match name {
        "Gtk" => Some(Box::new(gtk::GtkFactory {})),
        "Stdout" => Some(Box::new(StdoutFactory {})),
        _ => None,
    }
}

pub struct Stdout {
    projects: Vec<Project>,
}

struct StdoutFactory {}

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
    fn run(&self) -> Option<Project> {
        println!("Please choose from one of next projects:");
        print_projects(&self.projects);
        println!("And rerun with `{} <project>`", env::args().next().unwrap());
        None
    }
}
