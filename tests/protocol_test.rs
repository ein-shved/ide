use ide::protocol::*;
use protobuf::Message as _;
use std::collections::VecDeque;
use std::{io, path::PathBuf};
use futures::join;

struct TestStreamError
{
}

impl Stream for TestStreamError
{
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        io::Result::Err(io::Error::new(io::ErrorKind::Unsupported, "TEST"))
    }

    async fn read(&mut self) -> io::Result<Message> {
        io::Result::Err(io::Error::new(io::ErrorKind::Unsupported, "TEST"))
    }
}

struct TestStream<W, R>
where
    W: FnMut(Message) -> io::Result<()>,
    R: FnMut() -> io::Result<Message>,
{
    w: W,
    r: R,
}

impl<W, R> Stream for TestStream<W, R>
where
    W: FnMut(Message) -> io::Result<()>,
    R: FnMut() -> io::Result<Message>,
{
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        (self.w)(msg)
    }

    async fn read(&mut self) -> io::Result<Message> {
        (self.r)()
    }
}

impl<W, R> TestStream<W, R>
where
    W: FnMut(Message) -> io::Result<()>,
    R: FnMut() -> io::Result<Message>,
{
    pub fn new(w: W, r: R) -> Self {
        Self { w, r }
    }
}

struct TestStreamHandy<W, R>
where
    W: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    w: W,
    r: R,
    cache: VecDeque<u8>,
    secs: Vec<u8>,
}

impl<W, R> Stream for TestStreamHandy<W, R>
where
    W: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        self.cache.append(&mut msg.into());
        if self.cache.len() < Frame::len() {
            return Ok(());
        }
        let frame = Frame::from(&self.cache);
        if self.cache.len() < Frame::len() + frame.len as usize {
            return Ok(());
        }
        self.secs.push(frame.seq_id);
        self.cache.drain(0..Frame::len());
        (self.w)(frame.typ, self.cache.drain(0..frame.len as usize).collect())
    }

    async fn read(&mut self) -> io::Result<Message> {
        let (typ, mut msg) = (self.r)()?;
        let seq_id = if typ == FrameType::Response {
            self.secs.pop().unwrap_or(1)
        } else {
            1
        };
        let frame = Frame::new(typ, seq_id, msg.len() as u16);
        let mut res: Message = frame.into();
        res.append(&mut msg);
        Ok(res)
    }
}

impl<W, R> TestStreamHandy<W, R>
where
    W: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    pub fn new(w: W, r: R) -> Self {
        Self {
            w,
            r,
            cache: VecDeque::<u8>::default(),
            secs: Vec::<u8>::default(),
        }
    }
}

fn make_test_projects() -> Vec<ide::Project> {
    vec![
        ide::Project {
            name: String::from("a"),
            path: "/a/a/a".into(),
            session_file: None,
            exists: true,
        },
        ide::Project {
            name: String::from("b"),
            path: "/b/b/b".into(),
            session_file: None,
            exists: true,
        },
    ]
}

#[tokio::test]
async fn client_list_projects() {
    let mut write_called = 0;
    let mut read_called = 0;
    let prjcts = make_test_projects();
    let write = |typ: FrameType, msg: Message| {
        let req = idep::Request::parse_from_bytes(&msg).unwrap();
        assert_eq!(FrameType::Request, typ);
        assert!(req.has_list_projects());
        write_called = write_called + 1;
        Ok(())
    };
    let read = || {
        read_called = read_called + 1;
        Ok((
            FrameType::Response,
            idep::ListProjectsResponse::from(&prjcts)
                .write_to_bytes()
                .unwrap(),
        ))
    };
    let mut client = Client::new(TestStreamHandy::new(write, read));
    let res = client.list_projects().await;
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(write_called, 1);
    assert_eq!(read_called, 1);
    assert_eq!(prjcts[0], res[0]);
    assert_eq!(prjcts[1], res[1]);
}

#[tokio::test]
async fn server_list_projects() {
    let prjcts = make_test_projects();
    let server = Server::new(TestStreamError{}, prjcts.clone());
    let res = server.list_projects();
    assert!(res.is_ok());
    let res = idep::ListProjectsResponse::parse_from_bytes(&res.unwrap());
    assert!(res.is_ok());
    assert_eq!(prjcts, Vec::<ide::Project>::from(res.unwrap()));
}

#[tokio::test]
async fn server_list_projects_next() {
    let mut write_called = 0;
    let mut read_called = 0;
    let prjcts = make_test_projects();
    let write = |typ: FrameType, msg: Message| {
        let rsp = idep::ListProjectsResponse::parse_from_bytes(&msg).unwrap();
        let rsp: Vec<ide::Project> = rsp.into();
        assert_eq!(FrameType::Response, typ);
        assert_eq!(prjcts[0], rsp[0]);
        assert_eq!(prjcts[1], rsp[1]);
        write_called = write_called + 1;
        Ok(())
    };
    let read = || -> io::Result<(FrameType, Message)> {
        read_called = read_called + 1;
        let mut req = idep::Request::new();
        req.set_list_projects(idep::request::ListProjects::new());
        Ok((FrameType::Request, req.write_to_bytes().unwrap()))
    };
    let mut server = Server::new(TestStreamHandy::new(write, read), prjcts.clone());
    assert!(server.next().await.is_ok());
    assert_eq!(write_called, 1);
    assert_eq!(read_called, 1);
}

#[tokio::test]
async fn server_client_list_projects() {
    let (left, right) = streams::VirtualStreamBuilder::new_streams();
    let prjcts = make_test_projects();

    let mut client = Client::new(left);
    let mut server = Server::new(right, prjcts.clone());

    let next = server.next();
    let res = client.list_projects();

    let (next, res) = join!(next, res);
    assert!(next.is_ok());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), prjcts);
}
