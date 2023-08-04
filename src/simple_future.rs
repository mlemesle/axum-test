// use std::future::{Future, PollFn};

// pub struct SimpleFuture<T, E, F: Future<Output = Result<T, E>>> {
//     f: F,
// }

// impl<T, E, F> Future for SimpleFuture<T, E, F>
// where
//     F: Future<Output = Result<T, E>>,
// {
//     type Output = Result<T, E>;

//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//     }
// }
