use crate::Project;
use std::iter::Iterator;

mod gtk;
mod stdout;
mod stdio;

pub trait Ui {
    fn run(&mut self) -> Option<Project>;
}

type Projects<'a> = &'a mut dyn Iterator<Item = Project>;

pub trait UiFactory {
    fn new<'a>(&self, projects: Projects<'a>) -> Box<dyn Ui>;
    fn preferred_editor(&self) -> Option<String>
    {
        None
    }
}

pub fn new() -> Box<dyn UiFactory> {
    Box::new(gtk::GtkFactory {})
}

pub fn from(name: &str) -> Option<Box<dyn UiFactory>> {
    match name {
        "Gtk" => Some(Box::new(gtk::GtkFactory {})),
        "Stdout" => Some(Box::new(stdout::StdoutFactory {})),
        "Stdio" => Some(Box::new(stdio::StdioFactory {})),
        _ => None,
    }
}

