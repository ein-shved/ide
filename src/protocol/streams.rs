use super::{Stream, Message};
use std::{rc::Rc, cell::RefCell};
use std::collections::VecDeque;
use std::io;
use super::{Frame, FrameType, idep};
use protobuf::Message as _;

type StreamQueue = Rc<RefCell<Vec<u8>>>;

#[derive(Default)]
pub struct VirtualStreamBuilder
{
    left: StreamQueue,
    right: StreamQueue
}

pub struct VirtualStream
{
    write_queue: StreamQueue,
    read_queue: StreamQueue,
}

impl Stream for VirtualStream {
    async fn write(&mut self, msg: Message) -> io::Result<()>
    {
        self.write_queue.borrow_mut().append(&mut msg.into());
        Ok(())
    }
    async fn read(&mut self) -> io::Result<Message>
    {
        let mut msg = Message::new();
        std::mem::swap(
            &mut msg,
            &mut *self.read_queue.borrow_mut());
        Ok(msg)
    }
}

impl VirtualStream {
    pub fn new(write_queue: StreamQueue, read_queue: StreamQueue) -> Self
    {
        Self { write_queue, read_queue }
    }
}

impl VirtualStreamBuilder {
    pub fn make_left(&self) -> VirtualStream
    {
        VirtualStream::new(self.left.clone(), self.right.clone())
    }

    pub fn make_right(&self) -> VirtualStream
    {
        VirtualStream::new(self.right.clone(), self.left.clone())
    }

    pub fn make(&self) -> (VirtualStream, VirtualStream)
    {
        (self.make_left(), self.make_right())
    }

    pub fn new_streams() ->(VirtualStream, VirtualStream)
    {
        VirtualStreamBuilder::default().make()
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

