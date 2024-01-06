use super::{idep, FrameType, Message, PackageReceiver, PackageSender, Receiver, Sender};
use protobuf::Message as _;
use tokio::{
    select,
    sync::{mpsc, oneshot},
};

use std::io;

type CachedRequest = oneshot::Sender<io::Result<Message>>;
pub trait OnRequestH = FnMut(idep::Request) -> io::Result<Message>;
pub trait OnUpdateH = FnMut(idep::OnUpdate) -> io::Result<()>;

enum Notice {
    RequestTask((idep::Request, CachedRequest)),
    UpdateTask(idep::OnUpdate),
}

pub struct BidirectStream<S, R>
where
    S: Sender,
    R: Receiver,
{
    sender: PackageSender<S>,
    receiver: PackageReceiver<R>,
    requests: std::collections::BTreeMap<u8, CachedRequest>,
    m_receiver: mpsc::Receiver<Notice>,
    m_sender: mpsc::Sender<Notice>,
}

impl<S, R> From<(S, R)> for BidirectStream<S, R>
where
    S: Sender,
    R: Receiver,
{
    fn from(value: (S, R)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl<S, R> BidirectStream<S, R>
where
    S: Sender,
    R: Receiver,
{
    pub fn new(sender: S, receiver: R) -> Self {
        let (tx, rx) = mpsc::channel(1024);
        Self {
            sender: sender.into(),
            receiver: receiver.into(),
            requests: std::collections::BTreeMap::<u8, CachedRequest>::default(),
            m_sender: tx.into(),
            m_receiver: rx,
        }
    }

    pub async fn next<Rq, Up>(&mut self, rq: Option<Rq>, up: Option<Up>) -> io::Result<()>
    where
        Rq: OnRequestH,
        Up: OnUpdateH,
    {
        let result: io::Result<()>;
        select! {
            Ok((typ, seq_id, msg)) = self.receiver.read_package() => result = match typ {
                FrameType::Response => self.process_response(seq_id, msg).await,
                FrameType::Request => self.process_request(seq_id, msg, rq).await,
                FrameType::Notify => self.process_update(msg, up).await,
            },
            Some(nt) = self.m_receiver.recv() => result = match nt {
                Notice::RequestTask(req) => self.send_request(req.0, req.1).await,
                Notice::UpdateTask(upd) => self.send_update(upd).await,
            },
        };
        result
    }

    pub fn get_sender(&self) -> BidirectSender {
        BidirectSender {
            sender: self.m_sender.clone(),
        }
    }

    async fn process_response(&mut self, seq_id: u8, msg: Message) -> io::Result<()> {
        let req = self.requests.remove(&seq_id);
        if let Some(req) = req {
            req.send(Ok(msg)).unwrap();
            Ok(())
        } else {
            io::Result::Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Wrong seq_id in response",
            ))
        }
    }

    async fn process_request<Rq: OnRequestH>(
        &mut self,
        seq_id: u8,
        msg: Message,
        rq: Option<Rq>,
    ) -> io::Result<()> {
        let req = idep::Request::parse_from_bytes(&msg)?;
        // TODO(Shvedov) process errors
        let rsp = rq.unwrap()(req)?;
        self.sender
            .write_package(FrameType::Response, seq_id, rsp)
            .await
    }

    async fn process_update<Up: OnUpdateH>(
        &mut self,
        msg: Message,
        up: Option<Up>,
    ) -> io::Result<()> {
        let upd = idep::OnUpdate::parse_from_bytes(&msg)?;
        up.unwrap()(upd)
    }

    async fn send_request(&mut self, req: idep::Request, cache: CachedRequest) -> io::Result<()> {
        let seq_id = self.sender.write_request(req).await?;
        self.requests.insert(seq_id, cache);
        Ok(())
    }

    async fn send_update(&mut self, upd: idep::OnUpdate) -> io::Result<()> {
        self.sender.write_update(upd).await.map(|_| ())
    }
}

pub struct BidirectSender {
    sender: mpsc::Sender<Notice>,
}

impl BidirectSender {
    pub async fn send_request(&mut self, req: idep::Request) -> io::Result<Message> {
        let (tx, rx) = oneshot::channel::<io::Result<Message>>();
        self.sender
            .send(Notice::RequestTask((req, tx)))
            .await
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn send_update(&mut self, upd: idep::OnUpdate) -> io::Result<()> {
        self.sender.send(Notice::UpdateTask(upd)).await.unwrap();
        Ok(())
    }
}
