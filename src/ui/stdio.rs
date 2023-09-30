use super::super::project::Project;
use super::{Projects, Ui, UiFactory};
use std::fmt;
use std::io;
use std::io::Write;

struct Stdio {
    projects: Vec<Project>,
}

pub struct StdioFactory {}

impl UiFactory for StdioFactory {
    fn new<'a>(&self, projects: Projects<'a>) -> Box<dyn Ui> {
        Box::new(Stdio {
            projects: projects.collect(),
        })
    }
}

const PROMPT: &str = "[rm] <number>|<name>|<path>|<C-D>:";
const REDCODE: &str = "\x1b[0;31m";
const RESETCODE: &str = "\x1b[0m";

enum Answer {
    Exit,
    Index(usize),
    Name(String),
    Path(String),
    Wrong(String),
    Remove(Box<Answer>),
}

impl Answer {
    fn from(line: &str) -> Self {
        let line = line.trim();
        if let Ok(index) = line.parse::<i32>() {
            if index <= 0 {
                Self::Wrong(String::from("Invalid index, shoud be greater then zero"))
            } else {
                Self::Index(index as usize)
            }
        } else if line == "" {
            Self::Exit
        } else if line.len() > 2 && &line[0..2] == "rm" {
            let a = Answer::from(&line[3..]);
            if matches!(a, Self::Remove(_)) {
                Self::Wrong(String::from("rm rm?? Wot??"))
            } else if matches!(a, Self::Exit) {
                Self::Wrong(String::from("Please consider to specify project"))
            } else {
                Self::Remove(Box::new(a))
            }
        } else if line.contains("/") {
            Self::Path(String::from(line))
        } else {
            Self::Name(String::from(line))
        }
    }
}

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Exit => write!(f, "Exit"),
            Self::Index(n) => write!(f, "{}", n),
            Self::Name(n) => write!(f, "{}", n),
            Self::Path(p) => write!(f, "{}", p),
            Self::Wrong(err) => write!(f, "{}", err),
            Self::Remove(other) => write!(f, "Remove {}", other),
        }
    }
}

#[cfg(test)]
mod answer_tests {
    use super::Answer;

    #[test]
    fn check_exit() {
        let a = Answer::from("");
        assert!(matches!(a, Answer::Exit));
        assert_eq!(a.to_string(), "Exit");
    }

    #[test]
    fn check_index() {
        let a = Answer::from("325");
        assert!(matches!(a, Answer::Index(325)));
        assert_eq!(a.to_string(), "325");
    }

    #[test]
    fn check_wrong() {
        let a = Answer::from("-325");
        assert!(matches!(a, Answer::Wrong(_)));
    }

    #[test]
    fn check_name() {
        let a = Answer::from("name");
        assert!(matches!(a, Answer::Name(_)));
        assert_eq!(a.to_string(), "name");
    }

    #[test]
    fn check_short_name() {
        let a = Answer::from("n");
        assert!(matches!(a, Answer::Name(_)));
        assert_eq!(a.to_string(), "n");
    }

    #[test]
    fn check_path() {
        let a = Answer::from("path/name");
        assert!(matches!(a, Answer::Path(_)));
        assert_eq!(a.to_string(), "path/name");
    }

    #[test]
    fn check_rm() {
        let a = Answer::from("rm path/name");
        assert!(matches!(a, Answer::Remove(_)));
        assert_eq!(a.to_string(), "Remove path/name");
    }

    #[test]
    fn check_rm_rm() {
        let a = Answer::from("rm rm path/name");
        assert!(matches!(a, Answer::Wrong(_)));
        assert_eq!(a.to_string(), "rm rm?? Wot??");
    }
}

impl Stdio {
    fn print_project(project: &Project, n: usize, shift: usize, nshift: usize) {
        println!(
            "\t[{n:<nshift$}] {redcode}{name:<shift$}{RESETCODE} at {path}",
            name = project.name,
            path = project.path.to_str().unwrap(),
            redcode = if project.exists { RESETCODE } else { REDCODE },
        )
    }

    fn get_answer() -> Answer {
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).map_or_else(
            |_| Answer::Exit,
            move |_| {
                println!();
                Answer::from(&answer)
            },
        )
    }

    fn print_projects(&self) {
        let shift = Project::max_name_length(&self.projects);
        let mut no_exists = false;
        let nshift = {
            let mut res = 1;
            let mut shift = 1;
            loop {
                if res * 10 > self.projects.len() {
                    break shift;
                };
                res *= 10;
                shift += 1;
            }
        };
        for proj in &self.projects {
            no_exists = no_exists || !proj.exists;
        }
        let no_exists = if no_exists {
            format!(" ({REDCODE}red names{RESETCODE} means unexisted paths)")
        } else {
            String::new()
        };

        println!("Please choose from one of next projects{}:", no_exists);
        for (n, project) in self.projects.iter().enumerate() {
            Self::print_project(&project, n + 1, shift, nshift);
        }
    }

    fn step(&self, print_projects: bool) -> Answer {
        if print_projects {
            self.print_projects();
        }
        print!("{PROMPT}");
        if io::stdout().flush().is_err() {
            Answer::Exit
        } else {
            Self::get_answer()
        }
    }

    fn from_index(&self, index: usize) -> Result<usize, String> {
        if let Some(_) = self.projects.get(index - 1) {
            Ok(index - 1)
        } else {
            Err(format!("No such index: {index}"))
        }
    }

    fn from_name(&self, name: &str) -> Result<usize, String> {
        for (index, proj) in self.projects.iter().enumerate() {
            if proj.name == name {
                return Ok(index);
            }
        }
        Err(format!("No projects with name {name} found"))
    }

    fn from_path(&mut self, path: &str, allow_create: bool) -> Result<usize, String> {
        for (index, proj) in self.projects.iter().enumerate() {
            if proj.path.to_str().unwrap() == path {
                return Ok(index);
            }
        }
        if allow_create {
            self.projects.push(Project::from_path(path));
            Ok(self.projects.len() - 1)
        } else {
            Err(format!("No projects at '{path}' exists"))
        }
    }

    fn remove(&mut self, index: usize) -> String {
        let proj = &self.projects[index];
        match proj.rm() {
            Err(what) => format!(
                "Failed to remove project {}: {}",
                proj.name,
                what.to_string()
            ),
            Ok(_) => {
                self.projects.remove(index);
                String::new()
            }
        }
    }

    fn remove_answer(&mut self, ans: &Answer) -> String {
        match self.from_answer(ans, false) {
            Ok(proj) => {
                self.remove(proj);
                String::new()
            }
            Err(what) => what,
        }
    }

    fn from_answer(&mut self, ans: &Answer, allow_create: bool) -> Result<usize, String> {
        match ans {
            Answer::Index(i) => self.from_index(*i),
            Answer::Name(name) => self.from_name(name),
            Answer::Path(path) => self.from_path(path, allow_create),
            _ => Err(format!("{}", ans)),
        }
    }
}

impl Ui for Stdio {
    fn run(&mut self) -> Option<Project> {
        let mut n = 0;
        loop {
            let ans = self.step(n % 5 == 0);
            n += 1;
            let proj = match ans {
                Answer::Exit => break None,
                Answer::Remove(what) => {
                    n = 0;
                    Err(self.remove_answer(&what))
                },
                _ => self.from_answer(&ans, true),
            };
            match proj {
                Ok(i) => break Some(self.projects[i].clone()),
                Err(what) => {
                    if !what.is_empty() {
                        println!("Error: {what}")
                    }
                }
            };
        }
    }
}
