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

#[test]
fn test_math_work_is_executed() -> Result<()> {
    assert_eq!(0, compute_math_in_parallel(1, 2, 1, 2)?);
    assert_eq!(10, compute_math_in_parallel(5, 10, 2, 3)?);
    Ok(())
}

fn compute_math_in_parallel(f1: i32, f2: i32, t1: i32, t2: i32) -> Result<i32> {
    let (sender, receiver) = mpsc::channel();

    let pool = ThreadPool::new(2)?;
    let sender_clone = sender.clone();
    pool.execute(move || {
        let p = f1 * f2;
        sender_clone.send((1, p)).unwrap();
    });
    let sender_clone = sender.clone();
    pool.execute(move || {
        let s = t1 + t2;
        sender_clone.send((2, s)).unwrap();
    });

    drop(sender);

    let mut results: Vec<(i32, i32)> = receiver.iter().collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results[0].1 / results[1].1)
}