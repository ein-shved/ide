use ide::protocol::*;
use protobuf::Message as _;
use std::{io, path::PathBuf};

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

#[tokio::test]
async fn client_list_projects() {
    let mut write_called = 0;
    let mut read_called = 0;
    let prjcts = vec![
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
    ];
    let write = |msg: Message| {
        let req = idep::Request::parse_from_bytes(&msg).unwrap();
        assert!(req.has_list_projects());
        write_called = write_called + 1;
        Ok(())
    };
    let read = || {
        read_called = read_called + 1;
        Ok(idep::ListProjectsResponse::from(&prjcts).write_to_bytes().unwrap())
    };
    let mut client = Client::new(TestStream::new(write, read));
    let res = client.list_projects().await;
    assert_eq!(write_called, 1);
    assert_eq!(read_called, 1);
    assert_eq!(prjcts[0], res[0]);
    assert_eq!(prjcts[1], res[1]);
}

#[tokio::test]
async fn server_list_projects() {
    let mut write_called = 0;
    let mut read_called = 0;
    let prjcts = vec![
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
    ];
    let write = |msg: Message| {
        let rsp = idep::ListProjectsResponse::parse_from_bytes(&msg).unwrap();
        let rsp: Vec<ide::Project> = rsp.into();
        assert_eq!(prjcts[0], rsp[0]);
        assert_eq!(prjcts[1], rsp[1]);
        write_called = write_called + 1;
        Ok(())
    };
    let read = || -> io::Result<Message> {
        read_called = read_called + 1;
        io::Result::Err(io::Error::new(io::ErrorKind::Other, "TEST"))
    };
    let mut server = Server::new(TestStream::new(write, read), prjcts.clone());
    let res = server.list_projects().await;
    assert!(res.is_ok());
    assert_eq!(write_called, 1);
    assert_eq!(read_called, 0);
}
