pub struct ResultIterator<I: Iterator> {
    iterator: I,
    errored: bool,
}

impl<I: Iterator> ResultIterator<I> {
    pub fn new(iterator: I) -> ResultIterator<I> {
        ResultIterator {
            iterator: iterator,
            errored: false,
        }
    }
}

impl<T, E, I: Iterator<Item=Result<T, E>>> Iterator for ResultIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored {
            return None
        }
        let value = self.iterator.next();
        if let Some(Err(..)) = value {
            self.errored = true
        }
        value
    }
}
