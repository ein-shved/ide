include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

pub mod streams;

use crate::Project;
use byteorder::{ByteOrder as _, NetworkEndian as NE};
use idep::{Response, Request};
use protobuf::Message as _;
use std::io;
use streams::{BidirectSender, BidirectStream};

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
where
    C: std::ops::Index<usize, Output = u8>,
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

pub trait Sender {
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

impl From<&idep::response::ListProjects> for Projects {
    fn from(value: &idep::response::ListProjects) -> Self {
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

impl From<&Projects> for idep::response::ListProjects {
    fn from(value: &Projects) -> Self {
        let mut s = Self::new();
        for proj in value.iter() {
            s.projects.push(proj.into())
        }
        s
    }
}

impl From<&Projects> for Response {
    fn from(value: &Projects) -> Self {
        let mut s = Self::new();
        s.set_list_projects(value.into());
        s
    }
}

pub struct Client<S: Sender, R: Receiver> {
    stream: BidirectStream<S, R>,
}

impl<S: Sender, R: Receiver> From<(S, R)> for Client<S, R> {
    fn from(value: (S, R)) -> Self {
        Self::new(value.0, value.1)
    }
}

pub struct ClientRequester {
    sender: BidirectSender,
}

impl<S: Sender, R: Receiver> Client<S, R> {
    pub fn new(s: S, r: R) -> Self {
        Self {
            stream: BidirectStream::new(s, r),
        }
    }

    pub fn get_requester(&self) -> ClientRequester {
        ClientRequester {
            sender: self.stream.get_sender(),
        }
    }

    pub async fn go_loop(&mut self) -> io::Result<()> {
        self.stream
            .go_loop(
                Some(|req| Self::on_request(req)),
                Some(|up| Self::on_update(up)),
            )
            .await
    }

    fn on_request(_req: idep::Request) -> io::Result<idep::Response> {
        io::Result::Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Not implemented yet",
        ))
    }

    fn on_update(_upd: idep::OnUpdate) -> io::Result<()> {
        Ok(()) // TODO (Shvedov)
    }
}

impl ClientRequester {
    pub async fn list_projects(&mut self) -> io::Result<Projects> {
        let mut req = Request::new();
        req.set_list_projects(idep::request::ListProjects::new());
        let rsp = self.sender.send_request(req).await?;
        let rsp = Response::parse_from_bytes(&rsp)?;
        if !rsp.has_list_projects() {
            Err(io::Error::new(io::ErrorKind::InvalidData,
                "Response does not contains 'list_projects' field"))
        } else {
            Ok(rsp.list_projects().into())
        }
    }
}

pub struct Server<S: Sender, R: Receiver> {
    stream: BidirectStream<S, R>,
    projects: Projects,
}

impl<S: Sender, R: Receiver> From<(S, R)> for Server<S, R> {
    fn from(value: (S, R)) -> Self {
        Self::new(value, Default::default())
    }
}

impl<S: Sender, R: Receiver> Server<S, R> {
    pub fn new(stream: (S, R), projects: Projects) -> Self {
        Self {
            stream: BidirectStream::new(stream.0, stream.1),
            projects,
        }
    }

    pub fn list_projects(&self) -> io::Result<Message> {
        let rsp: Response = (&self.projects).into();
        Ok(rsp.write_to_bytes()?)
    }

    pub async fn next(&mut self) -> io::Result<()> {
        self.stream
            .go_loop(
                Some(|req| Self::on_request(req, &mut self.projects)),
                Some(|up| Self::on_update(up)),
            )
            .await
    }

    fn on_request(req: idep::Request, prj: &mut Projects) -> io::Result<idep::Response> {
        if req.has_list_projects() {
            Ok(Response::from(prj.as_ref()))
        } else {
            io::Result::Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Not implemented yet",
            ))
        }
    }

    fn on_update(_upd: idep::OnUpdate) -> io::Result<()> {
        Ok(()) // TODO (Shvedov)
    }
}
