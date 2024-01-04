include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

pub mod streams;

use crate::Project;
use byteorder::{ByteOrder as _, NetworkEndian as NE};
use idep::{ListProjectsResponse, Request};
use protobuf::Message as _;
use std::io;
use streams::BidirectStream;

pub type Message = Vec<u8>;
type Projects = Vec<Project>;

#[derive(PartialEq, Debug)]
pub enum FrameType {
    Request = 1,
    Response = 2,
    Notify = 0,
}

pub struct Frame {
    pub typ: FrameType,
    pub seq_id: u8,
    pub len: u16,
}

impl Frame {
    pub fn new(typ: FrameType, seq_id: u8, len: u16) -> Self {
        Self { typ, seq_id, len }
    }
    pub fn len() -> usize {
        4
    }
}

impl<C> From<&C> for Frame
    where C: std::ops::Index<usize, Output = u8>
{
    fn from(value: &C) -> Self {
        let len = vec![value[2], value[3]];
        Self::new(value[0].into(), value[1], NE::read_u16(&len))
    }
}

impl From<Frame> for Message {
    fn from(value: Frame) -> Self {
        let mut res = vec![value.typ.into(), value.seq_id, 0, 0];
        NE::write_u16(&mut res[2..4], value.len);
        res
    }
}

impl From<u8> for FrameType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Request,
            2 => Self::Response,
            0 => Self::Notify,
            _ => panic!("Invalid value of frame type {value}"),
        }
    }
}
impl From<FrameType> for u8 {
    fn from(value: FrameType) -> Self {
        match value {
            FrameType::Request => 1,
            FrameType::Response => 2,
            FrameType::Notify => 0,
        }
    }
}

pub trait  Sender {
    async fn send(&mut self, msg: Message) -> io::Result<()>;
}

pub trait Receiver {
    async fn recv(&mut self) -> io::Result<Message>;
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

pub struct Client<S: Sender, R: Receiver> {
    stream: BidirectStream<S, R>,
}

impl<S: Sender, R: Receiver> Client<S, R> {
    pub fn new(s: S, r: R) -> Self {
        Self { stream: BidirectStream::new(s, r) }
    }

    //pub async fn list_projects(&mut self) -> io::Result<Projects> {
    //    let mut req = Request::new();
    //    req.set_list_projects(idep::request::ListProjects::new());
    //    let send_seq = self.stream.write_request(req).await?;
    //    let (typ, seq, rsp) = self.stream.read_package().await?;
    //    if seq != send_seq || typ != FrameType::Response {
    //        panic!("Unexpected response")
    //    }
    //    let rsp = ListProjectsResponse::parse_from_bytes(&rsp)?;
    //    Ok(rsp.into())
    //}
}

pub struct Server<S: Sender, R: Receiver> {
    stream: BidirectStream<S, R>,
    projects: Projects,
}

impl<S: Sender, R: Receiver> Server<S, R> {
    pub fn new(s: S, r: R, projects: Projects) -> Self {
        Self {
            stream: BidirectStream::new(s, r),
            projects,
        }
    }

    pub fn list_projects(&self) -> io::Result<Message> {
        let rsp: ListProjectsResponse = (&self.projects).into();
        Ok(rsp.write_to_bytes()?)
    }

    //pub async fn next(&mut self) -> io::Result<()> {
    //    let (typ, seq, msg) = self.stream.read_package().await?;
    //    if typ == FrameType::Request {
    //        let req = Request::parse_from_bytes(&msg)?;
    //        if req.has_list_projects() {
    //            self.stream.send_response(seq, self.list_projects()?).await?;
    //        }
    //        Ok(())
    //    } else {
    //        Ok(()) // TODO(Shvedov)
    //    }
    //}
}
