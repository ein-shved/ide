use tokio::sync::oneshot;

trait AsyncCallback<A, R>
{
    async fn call(&self, args: A) -> R;
}

trait AsyncCallbackCaller<A, R>
{
    fn call(&self, args: A) -> oneshot::Receiver<R>;
}

struct AsyncCallbackCallerImpl<A, R, Cb: AsyncCallback<A, R>>
{
    cb: Cb
}

impl<A, R, Cb> From<Cb> for AsyncCallbackCallerImpl<A, R, Cb>
    where Cb: AsyncCallback<A, R>
{
    fn from(cb: Cb) -> Self {
        Self { cb }
    }
}

impl<A, R, Cb> AsyncCallbackCaller<A, R> for AsyncCallbackCallerImpl<A, R, Cb>
    where Cb: AsyncCallback<A, R>
{
    fn call(&self, args: A) -> oneshot::Receiver<R> {

    }
}

struct AsyncCallbackStore<A, R>
{


}
