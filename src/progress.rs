use std::{io::stderr, sync::Arc};

use prodash::{
    render::line::{JoinHandle, Options, StreamKind, render},
    tree::Root,
};

pub fn progress() -> (Arc<Root>, JoinHandle) {
    let prodash: Arc<Root> = prodash::tree::root::Options {
        message_buffer_capacity: 200,
        ..Default::default()
    }
    .into();

    let handle = render(
        stderr(),
        Arc::downgrade(&prodash),
        Options {
            throughput: true,
            frames_per_second: 6.0,
            ..Options::default()
        }
        .auto_configure(StreamKind::Stderr),
    );

    (prodash, handle)
}
