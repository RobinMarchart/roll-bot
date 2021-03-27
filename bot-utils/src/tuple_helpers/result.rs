pub trait ResultChain<E> {
    type Output;
    fn result(self) -> Result<Self::Output, E>;
}

impl<O, E> ResultChain<E> for Result<O, E> {
    type Output = O;

    fn result(self) -> Result<Self::Output, E> {
        self
    }
}
