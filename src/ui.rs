use crate::Project;
use std::env;
use std::io;
use std::iter::Iterator;

pub trait Ui {
    fn run(&self) -> io::Result<()>;
}

pub fn new<Projects: Iterator<Item = Project>>(projects: Projects) -> Box<dyn Ui> {
    Box::new(Stdout::new(projects))
}

pub struct Stdout {
    projects: Vec<Project>,
}

fn get_shift(projects: &Vec<Project>) -> usize {
    let mut max_shift = 0;
    for proj in projects {
        if max_shift < proj.name.len() {
            max_shift = proj.name.len()
        }
    }
    max_shift
}

fn print_project(project: &Project, shift: usize) {
    println!(
        "\t{name:<shift$} at {path}",
        name = project.name,
        path = project.path.to_str().unwrap()
    )
}

fn print_projects(projects: &Vec<Project>) {
    let shift = get_shift(projects);

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
