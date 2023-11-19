include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use crate::Project;
use idep::{ListProjectsResponse, Request};
use protobuf::{Enum, Message as _};
use std::io;

pub type Message = Vec<u8>;
type Projects = Vec<Project>;

pub trait Stream {
    async fn write(&mut self, msg: Message) -> io::Result<()>;
    async fn read(&mut self) -> io::Result<Message>;
}

impl From<idep::Project> for Project {
    fn from(value: idep::Project) -> Self {
        Self {
            name: value.name,
            path: value.path.into(),
            session_file: None,
            exists: true,
        }
    }
}

impl From<ListProjectsResponse> for Projects {
    fn from(value: ListProjectsResponse) -> Self {
        let it = value
            .projects
            .iter()
            .map(|proj| Project::from(proj.clone()));
        it.collect()
    }
}

impl From<&Project> for idep::Project {
    fn from(value: &Project) -> Self {
        let mut s = Self::new();
        s.name = value.name.clone();
        s.path = value.path.to_str().unwrap().into();
        s
    }
}

impl From<&Projects> for ListProjectsResponse {
    fn from(value: &Projects) -> Self {
        let mut s = Self::new();
        for proj in value.iter() {
            s.projects.push(proj.into())
        }
        s
    }
}

pub struct Client<S: Stream> {
    stream: S,
}

impl<S: Stream> Client<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub async fn list_projects(&mut self) -> Projects {
        let mut req = Request::new();
        req.set_list_projects(idep::request::ListProjects::new());
        let _ = self.stream.write(req.write_to_bytes().unwrap()).await;
        let rsp = self.stream.read().await.unwrap();
        let rsp = ListProjectsResponse::parse_from_bytes(&rsp).unwrap();
        rsp.into()
    }
}

pub struct Server<S: Stream> {
    stream: S,
    projects: Projects,
}

impl<S: Stream> Server<S> {
    pub fn new(stream: S, projects: Projects) -> Self {
        Self { stream, projects }
    }

    pub async fn list_projects(&mut self) -> io::Result<()> {
        let rsp :ListProjectsResponse = (&self.projects).into();
        self.stream.write(rsp.write_to_bytes()?).await
    }
}
