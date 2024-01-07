use futures::{join, select};
use ide::protocol::*;
use protobuf::Message as _;
use std::collections::VecDeque;
use std::{cell::RefCell, io, rc::Rc};

fn mk_test_error<T>() -> io::Result<T> {
    io::Result::Err(io::Error::new(io::ErrorKind::Unsupported, "TEST"))
}

struct TestSenderError {}
impl Sender for TestSenderError {
    async fn send(&mut self, _: Message) -> io::Result<()> {
        mk_test_error()
    }
}

struct TestReceiverError {}
impl Receiver for TestReceiverError {
    async fn recv(&mut self) -> io::Result<Message> {
        mk_test_error()
    }
}

struct TestSender<S>
where
    S: FnMut(Message) -> io::Result<()>,
{
    s: S,
}

impl<S> Sender for TestSender<S>
where
    S: FnMut(Message) -> io::Result<()>,
{
    async fn send(&mut self, msg: Message) -> io::Result<()> {
        (self.s)(msg)
    }
}

impl<S> From<S> for TestSender<S>
where
    S: FnMut(Message) -> io::Result<()>,
{
    fn from(s: S) -> Self {
        Self { s }
    }
}

struct TestReceiver<R>
where
    R: FnMut() -> io::Result<Message>,
{
    r: R,
}

impl<R> Receiver for TestReceiver<R>
where
    R: FnMut() -> io::Result<Message>,
{
    async fn recv(&mut self) -> io::Result<Message> {
        (self.r)()
    }
}

impl<R> From<R> for TestReceiver<R>
where
    R: FnMut() -> io::Result<Message>,
{
    fn from(r: R) -> Self {
        Self { r }
    }
}

struct TestSenderHandy<S>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
{
    s: S,
    cache: VecDeque<u8>,
    pub secs: Rc<RefCell<Vec<u8>>>,
}

impl<S> Sender for TestSenderHandy<S>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
{
    async fn send(&mut self, msg: Message) -> io::Result<()> {
        self.cache.append(&mut msg.into());
        if self.cache.len() < Frame::len() {
            return Ok(());
        }
        let frame = Frame::from(&self.cache);
        if self.cache.len() < Frame::len() + frame.len as usize {
            return Ok(());
        }
        self.secs.borrow_mut().push(frame.seq_id);
        self.cache.drain(0..Frame::len());
        (self.s)(frame.typ, self.cache.drain(0..frame.len as usize).collect())
    }
}

impl<S> From<S> for TestSenderHandy<S>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
{
    fn from(w: S) -> Self {
        Self {
            s: w,
            cache: VecDeque::<u8>::default(),
            secs: Rc::new(RefCell::new(Vec::<u8>::default())),
        }
    }
}

struct TestReceiverHandy<R>
where
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    r: R,
    secs: Rc<RefCell<Vec<u8>>>,
}

impl<R> Receiver for TestReceiverHandy<R>
where
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    async fn recv(&mut self) -> io::Result<Message> {
        let (typ, mut msg) = (self.r)()?;
        let seq_id = if typ == FrameType::Response {
            self.secs.borrow_mut().pop().unwrap_or(1)
        } else {
            1
        };
        let frame = Frame::new(typ, seq_id, msg.len() as u16);
        let mut res: Message = frame.into();
        res.append(&mut msg);
        Ok(res)
    }
}

impl<R> TestReceiverHandy<R>
where
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    pub fn new(r: R, secs: Rc<RefCell<Vec<u8>>>) -> Self {
        Self { r, secs }
    }
}

fn make_handy_pair<S, R>(s: S, r: R) -> (TestSenderHandy<S>, TestReceiverHandy<R>)
where
    R: FnMut() -> io::Result<(FrameType, Message)>,
    S: FnMut(FrameType, Message) -> io::Result<()>,
{
    let sender = TestSenderHandy::from(s);
    let receiver = TestReceiverHandy::new(r, sender.secs.clone());
    (sender, receiver)
}

fn make_error_pair() -> (TestSenderError, TestReceiverError) {
    (TestSenderError {}, TestReceiverError {})
}

fn make_bidir_stream<S, R>(
    s: S,
    r: R,
) -> streams::BidirectStream<TestSenderHandy<S>, TestReceiverHandy<R>>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    make_handy_pair(s, r).into()
}

fn make_handy_client<S, R>(s: S, r: R) -> Client<TestSenderHandy<S>, TestReceiverHandy<R>>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    make_handy_pair(s, r).into()
}

fn make_handy_server<S, R>(s: S, r: R) -> Server<TestSenderHandy<S>, TestReceiverHandy<R>>
where
    S: FnMut(FrameType, Message) -> io::Result<()>,
    R: FnMut() -> io::Result<(FrameType, Message)>,
{
    Server::new(make_handy_pair(s, r), make_test_projects())
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
            idep::Response::from(&prjcts)
                .write_to_bytes()
                .unwrap(),
        ))
    };
    let mut client = make_handy_client(write, read);
    let mut requester = client.get_requester();
    let res = requester.list_projects();
    let (_, res) = tokio::join!(client.go_loop(), res);
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
    let server = Server::new(make_error_pair(), prjcts.clone());
    let res = server.list_projects();
    assert!(res.is_ok());
    let res = idep::Response::parse_from_bytes(&res.unwrap()).unwrap();
    let res = res.list_projects();
    assert_eq!(prjcts, Vec::<ide::Project>::from(res));
}

#[tokio::test]
async fn server_list_projects_next() {
    let mut write_called = 0;
    let mut read_called = 0;
    let prjcts = make_test_projects();
    let write = |typ: FrameType, msg: Message| {
        let rsp = idep::Response::parse_from_bytes(&msg).unwrap();
        let rsp = rsp.list_projects();
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
    let mut server = make_handy_server(write, read);
    assert!(server.next().await.is_ok());
    assert_eq!(write_called, 1);
    assert_eq!(read_called, 1);
}

#[tokio::test]
async fn server_client_list_projects() {
    let (left, right) = streams::VirtualStreamBuilder::new_streams();
    let prjcts = make_test_projects();

    let client = Client::from(left);
    let mut requester = client.get_requester();
    let mut server = Server::new(right, prjcts.clone());

    let next = server.next();
    let res = requester.list_projects();

    let (next, res) = join!(next, res);
    assert!(next.is_ok());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), prjcts);
}

#[tokio::test]
async fn bidir_stream_request() {
    let mut request = idep::Request::new();
    request.set_list_projects(idep::request::ListProjects::new());
    let mut on_req_called = 0;
    let mut on_upd_called = 0;
    let mut on_wr_called = 0;
    let mut on_rd_called = 0;
    let prjcts = make_test_projects();

    let mut stream = make_bidir_stream(
        |typ, msg| {
            on_wr_called += 1;
            assert_eq!(typ, FrameType::Request);
            assert_eq!(idep::Request::parse_from_bytes(&msg).unwrap(), request);
            Ok(())
        },
        || {
            on_rd_called += 1;
            Ok((
                FrameType::Response,
                idep::Response::from(&prjcts)
                    .write_to_bytes()
                    .unwrap(),
            ))
        },
    );
    let mut sender = stream.get_sender();
    let on_req = |_| {
        on_req_called += 1;
        mk_test_error::<idep::Response>()
    };
    let on_upd = |_| {
        on_upd_called += 1;
        mk_test_error::<()>()
    };

//    let rsp = sender.send_request(request.clone());
//    let next = stream.go_loop(Some(on_req), Some(on_upd));


    tokio::select! {
        rsp = sender.send_request(request.clone()) => {
            assert!(rsp.is_ok());
            let rsp = idep::Response::parse_from_bytes(&rsp.unwrap());
            assert_eq!(rsp.unwrap(), idep::Response::from(&prjcts));
        },
        _ = stream.go_loop(Some(on_req), Some(on_upd)) => {
            panic!("Stream should never stop looping!");
        }
    }
}
