extern crate futures;

use std::any::Any;

use futures::sync::oneshot;
use futures::stream::futures_ordered;
use futures::prelude::*;

mod support;

#[test]
fn works_1() {
    let (a_tx, a_rx) = oneshot::channel::<u32>();
    let (b_tx, b_rx) = oneshot::channel::<u32>();
    let (c_tx, c_rx) = oneshot::channel::<u32>();

    let stream = futures_ordered(vec![a_rx, b_rx, c_rx]);

    let mut spawn = futures::executor::spawn(stream);
    b_tx.send(99).unwrap();
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());

    a_tx.send(33).unwrap();
    c_tx.send(33).unwrap();
    assert_eq!(Some(Ok(33)), spawn.wait_stream());
    assert_eq!(Some(Ok(99)), spawn.wait_stream());
    assert_eq!(Some(Ok(33)), spawn.wait_stream());
    assert_eq!(None, spawn.wait_stream());
}

#[test]
fn works_2() {
    let (a_tx, a_rx) = oneshot::channel::<u32>();
    let (b_tx, b_rx) = oneshot::channel::<u32>();
    let (c_tx, c_rx) = oneshot::channel::<u32>();

    let stream = futures_ordered(vec![
        Box::new(a_rx) as Box<Future<Item = _, Error = _>>,
        Box::new(b_rx.join(c_rx).map(|(a, b)| a + b)),
    ]);

    let mut spawn = futures::executor::spawn(stream);
    a_tx.send(33).unwrap();
    b_tx.send(33).unwrap();
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_ready());
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());
    c_tx.send(33).unwrap();
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_ready());
}

#[test]
fn from_iterator() {
    use futures::future::ok;
    use futures::stream::FuturesOrdered;

    let stream = vec![
        ok::<u32, ()>(1),
        ok::<u32, ()>(2),
        ok::<u32, ()>(3)
    ].into_iter().collect::<FuturesOrdered<_>>();
    assert_eq!(stream.len(), 3);
    assert_eq!(stream.collect().wait(), Ok(vec![1,2,3]));
}

#[test]
fn queue_never_unblocked() {
    let (_a_tx, a_rx) = oneshot::channel::<Box<Any+Send>>();
    let (b_tx, b_rx) = oneshot::channel::<Box<Any+Send>>();
    let (c_tx, c_rx) = oneshot::channel::<Box<Any+Send>>();

    let stream = futures_ordered(vec![
        Box::new(a_rx) as Box<Future<Item = _, Error = _>>,
        Box::new(b_rx.select(c_rx).then(|res| Ok(Box::new(res) as Box<Any+Send>))),
    ]);

    let mut spawn = futures::executor::spawn(stream);
    for _ in 0..10 {
        assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());
    }

    b_tx.send(Box::new(())).unwrap();
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());
    c_tx.send(Box::new(())).unwrap();
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());
    assert!(spawn.poll_stream_notify(&support::notify_noop(), 0).unwrap().is_not_ready());
}
