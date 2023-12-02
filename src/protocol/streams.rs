use super::{idep, Frame, FrameType};
use super::{Message, Stream};
use protobuf::Message as _;
use std::io;
use std::{cell::RefCell, rc::Rc};
use std::collections::VecDeque;
use tokio::sync::mpsc;

pub type VirtualStream = Rc<RefCell<VirtualStreamQueue>>;

pub struct VirtualStreamBuilder {
    left: VirtualStream,
    right: VirtualStream,
}

pub struct VirtualStreamQueue {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}


impl Stream for VirtualStreamQueue {
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        self.tx.send(msg).await.unwrap();
        Ok(())
    }
    async fn read(&mut self) -> io::Result<Message> {
        Ok(self.rx.recv().await.unwrap())
    }
}

impl Stream for VirtualStream {
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        self.borrow_mut().write(msg).await
    }
    async fn read(&mut self) -> io::Result<Message> {
        self.borrow_mut().read().await
    }
}

impl VirtualStreamBuilder {
    pub fn new() -> VirtualStreamBuilder {
        let (ltx, lrx) = mpsc::channel::<Message>(16);
        let (rtx, rrx) = mpsc::channel::<Message>(16);
        VirtualStreamBuilder {
            left: Rc::new(RefCell::new(VirtualStreamQueue { tx: ltx, rx: rrx })),
            right: Rc::new(RefCell::new(VirtualStreamQueue { tx: rtx, rx: lrx })),
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

pub struct HandyStream<S: Stream> {
    stream: S,
    cache: VecDeque<u8>,
}

impl<S: Stream> From<S> for HandyStream<S> {
    fn from(stream: S) -> Self {
        Self {
            stream,
            cache: VecDeque::<u8>::default(),
        }
    }
}

impl<S: Stream> HandyStream<S> {
    async fn get_more(&mut self) -> io::Result<()> {
        self.cache.append(&mut self.stream.read().await?.into());
        Ok(())
    }

    pub async fn read_exact(&mut self, len: usize) -> io::Result<Message> {
        while self.cache.len() < len {
            self.get_more().await?;
        }
        Ok(self.cache.drain(0..len).collect())
    }
}

impl<S: Stream> Stream for HandyStream<S> {
    async fn read(&mut self) -> io::Result<Message> {
        while self.cache.len() <= 1 {
            self.get_more().await?;
        }
        Ok(self.cache.drain(..).collect())
    }
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        self.stream.write(msg).await
    }
}

pub struct PackageStream<S: Stream> {
    stream: HandyStream<S>,
    seq_id: u8,
}

impl<S: Stream> Stream for PackageStream<S> {
    async fn read(&mut self) -> io::Result<Message> {
        self.stream.read().await
    }
    async fn write(&mut self, msg: Message) -> io::Result<()> {
        self.stream.write(msg).await
    }
}

impl<S: Stream> From<HandyStream<S>> for PackageStream<S> {
    fn from(stream: HandyStream<S>) -> Self {
        Self { stream, seq_id: 0 }
    }
}

impl<S: Stream> From<S> for PackageStream<S> {
    fn from(stream: S) -> Self {
        Self {
            stream: stream.into(),
            seq_id: 0,
        }
    }
}

impl<S: Stream> PackageStream<S> {
    pub async fn read_package(&mut self) -> io::Result<(FrameType, u8, Message)> {
        let frame = self.stream.read_exact(Frame::len()).await?;
        let frame = Frame::from(&frame);
        let msg = self.stream.read_exact(frame.len as usize).await?;
        Ok((frame.typ, frame.seq_id, msg))
    }
    pub async fn write_package(
        &mut self,
        typ: FrameType,
        seq_id: u8,
        msg: Message,
    ) -> io::Result<()> {
        self.stream
            .write(Frame::new(typ, seq_id, msg.len() as u16).into())
            .await?;
        self.stream.write(msg).await?;
        Ok(())
    }
    pub async fn write_request(&mut self, req: idep::Request) -> io::Result<u8> {
        if self.seq_id >= 0xff {
            self.seq_id = 0;
        }
        self.seq_id += 1;
        let seq_id = self.seq_id;
        self.write_package(FrameType::Request, seq_id, req.write_to_bytes()?)
            .await?;
        Ok(seq_id)
    }
    pub async fn send_response(&mut self, seq_id: u8, rsp: Message) -> io::Result<()> {
        self.write_package(FrameType::Response, seq_id, rsp).await
    }
}
