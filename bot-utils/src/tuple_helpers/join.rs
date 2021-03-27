use super::ResultChain;
use async_trait::async_trait;
use tokio::task::{JoinError, JoinHandle};

#[async_trait]
pub trait JoinChain: Send {
    type Output: ResultChain<JoinError>;
    async fn join(self) -> Self::Output;
}

#[async_trait]
impl<T> JoinChain for JoinHandle<T>
where
    T: Send,
{
    type Output = Result<T, JoinError>;

    async fn join(self) -> Self::Output {
        self.await
    }
}
