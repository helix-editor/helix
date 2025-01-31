use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::{dispatch, events, register_dynamic_hook, register_event, register_hook};
#[test]
fn smoke_test() {
    events! {
        Event1 { content: String }
        Event2 { content: usize }
    }
    register_event::<Event1>();
    register_event::<Event2>();

    // setup hooks
    let res1: Arc<Mutex<String>> = Arc::default();
    let acc = Arc::clone(&res1);
    register_hook!(move |event: &mut Event1| {
        acc.lock().push_str(&event.content);
        Ok(())
    });
    let res2: Arc<AtomicUsize> = Arc::default();
    let acc = Arc::clone(&res2);
    register_hook!(move |event: &mut Event2| {
        acc.fetch_add(event.content, Ordering::Relaxed);
        Ok(())
    });

    // triggers events
    let thread = std::thread::spawn(|| {
        for i in 0..1000 {
            dispatch(Event2 { content: i });
        }
    });
    std::thread::sleep(Duration::from_millis(1));
    dispatch(Event1 {
        content: "foo".to_owned(),
    });
    dispatch(Event2 { content: 42 });
    dispatch(Event1 {
        content: "bar".to_owned(),
    });
    dispatch(Event1 {
        content: "hello world".to_owned(),
    });
    thread.join().unwrap();

    // check output
    assert_eq!(&**res1.lock(), "foobarhello world");
    assert_eq!(
        res2.load(Ordering::Relaxed),
        42 + (0..1000usize).sum::<usize>()
    );
}

#[test]
fn dynamic() {
    events! {
        Event3 {}
        Event4 { count: usize }
    };
    register_event::<Event3>();
    register_event::<Event4>();

    let count = Arc::new(AtomicUsize::new(0));
    let count1 = count.clone();
    let count2 = count.clone();
    register_dynamic_hook(
        move || {
            count1.fetch_add(2, Ordering::Relaxed);
            Ok(())
        },
        "Event3",
    )
    .unwrap();
    register_dynamic_hook(
        move || {
            count2.fetch_add(3, Ordering::Relaxed);
            Ok(())
        },
        "Event4",
    )
    .unwrap();
    dispatch(Event3 {});
    dispatch(Event4 { count: 0 });
    dispatch(Event3 {});
    assert_eq!(count.load(Ordering::Relaxed), 7)
}
