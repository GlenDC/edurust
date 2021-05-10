use std::sync::mpsc;
use webservice::thread::{ThreadPool, Result};

#[test]
fn test_work_is_executed() -> Result<()> {
    let (sender, receiver) = mpsc::channel();

    let pool = ThreadPool::new(2)?;
    let sender_clone = sender.clone();
    pool.execute(move || {
        sender_clone.send(1).unwrap();
    });
    let sender_clone = sender.clone();
    pool.execute(move || {
        sender_clone.send(2).unwrap();
    });
    let sender_clone = sender.clone();
    pool.execute(move || {
        sender_clone.send(3).unwrap();
    });

    drop(sender);

    let mut results: Vec<i32> = receiver.iter().collect();
    results.sort();
    assert_eq!(results, vec![1, 2, 3]);

    Ok(())
}
