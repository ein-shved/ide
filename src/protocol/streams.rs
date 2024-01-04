use super::{idep, Frame, FrameType};
use super::{Message, Receiver, Sender};
use protobuf::Message as _;
use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};
use std::io;
use tokio::sync::mpsc;

type MessageSender = mpsc::Sender<Message>;
type MessageReceiver = mpsc::Receiver<Message>;

pub type VirtualStream = Rc<RefCell<(MessageSender, MessageReceiver)>>;

pub struct VirtualStreamBuilder {
    left: VirtualStream,
    right: VirtualStream,
}

impl Sender for mpsc::Sender<Message> {
    async fn send(&mut self, msg: Message) -> io::Result<()> {
        Ok(Self::send(self, msg).await.unwrap())
    }
}

impl Receiver for mpsc::Receiver<Message> {
    async fn recv(&mut self) -> io::Result<Message> {
        Ok(Self::recv(self).await.unwrap())
    }
}

pub struct VirtualStreamQueue {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}

impl VirtualStreamBuilder {
    pub fn new() -> VirtualStreamBuilder {
        let (ltx, lrx) = mpsc::channel::<Message>(16);
        let (rtx, rrx) = mpsc::channel::<Message>(16);
        Self {
            left: Rc::new(RefCell::new((ltx, rrx))),
            right: Rc::new(RefCell::new((rtx, lrx))),
        }
    }
    pub fn make_left(&self) -> VirtualStream {
        self.left.clone()
    }

    pub fn make_right(&self) -> VirtualStream {
        self.right.clone()
    }

    pub fn make(&self) -> (VirtualStream, VirtualStream) {
        (self.make_left(), self.make_right())
    }

    pub fn new_streams() -> (VirtualStream, VirtualStream) {
        VirtualStreamBuilder::new().make()
    }
}

pub struct HandyReceiver<R: Receiver> {
    receiver: R,
    cache: VecDeque<u8>,
}

impl<R: Receiver> From<R> for HandyReceiver<R> {
    fn from(receiver: R) -> Self {
        Self {
            receiver,
            cache: VecDeque::<u8>::default(),
        }
    }
}

impl<R: Receiver> HandyReceiver<R> {
    async fn get_more(&mut self) -> io::Result<()> {
        self.cache.append(&mut self.receiver.recv().await?.into());
        Ok(())
    }

    pub async fn read_exact(&mut self, len: usize) -> io::Result<Message> {
        while self.cache.len() < len {
            self.get_more().await?;
        }
        Ok(self.cache.drain(0..len).collect())
    }
}

impl<R: Receiver> Receiver for HandyReceiver<R> {
    async fn recv(&mut self) -> io::Result<Message> {
        while self.cache.len() <= 1 {
            self.get_more().await?;
        }
        Ok(self.cache.drain(..).collect())
    }
}

pub struct PackageReceiver<R: Receiver> {
    receiver: HandyReceiver<R>,
}

impl<R: Receiver> Receiver for PackageReceiver<R> {
    async fn recv(&mut self) -> io::Result<Message> {
        self.receiver.recv().await
    }
}

impl<R: Receiver> From<HandyReceiver<R>> for PackageReceiver<R> {
    fn from(stream: HandyReceiver<R>) -> Self {
        Self { receiver: stream }
    }
}

impl<R: Receiver> From<R> for PackageReceiver<R> {
    fn from(stream: R) -> Self {
        Self {
            receiver: stream.into(),
        }
    }
}

impl<R: Receiver> PackageReceiver<R> {
    pub async fn read_package(&mut self) -> io::Result<(FrameType, u8, Message)> {
        let frame = self.receiver.read_exact(Frame::len()).await?;
        let frame = Frame::from(&frame);
        let msg = self.receiver.read_exact(frame.len as usize).await?;
        Ok((frame.typ, frame.seq_id, msg))
    }
}

pub struct PackageSender<S: Sender> {
    sender: S,
    seq_id: u8,
}

impl<S: Sender> From<S> for PackageSender<S> {
    fn from(value: S) -> Self {
        Self {
            sender: value,
            seq_id: 0,
        }
    }
}

impl<S: Sender> PackageSender<S> {
    pub async fn write_package(
        &mut self,
        typ: FrameType,
        seq_id: u8,
        msg: Message,
    ) -> io::Result<()> {
        self.sender
            .send(Frame::new(typ, seq_id, msg.len() as u16).into())
            .await?;
        self.sender.send(msg).await?;
        Ok(())
    }

    pub async fn write_request(&mut self, req: idep::Request) -> io::Result<u8> {
        let seq_id = self.next_seq_id();
        self.write_package(FrameType::Request, seq_id, req.write_to_bytes()?)
            .await?;
        Ok(seq_id)
    }

    pub async fn write_update(&mut self, upd: idep::OnUpdate) -> io::Result<u8> {
        let seq_id = self.next_seq_id();
        self.write_package(FrameType::Notify, seq_id, upd.write_to_bytes()?)
            .await?;
        Ok(seq_id)
    }

    pub async fn send_response(&mut self, seq_id: u8, rsp: Message) -> io::Result<()> {
        self.write_package(FrameType::Response, seq_id, rsp).await
    }

    fn next_seq_id(&mut self) -> u8 {
        if self.seq_id >= 0xff {
            self.seq_id = 0;
        }
        self.seq_id += 1;
        self.seq_id
    }
}

mod bidir;

pub use bidir::BidirectStream;
