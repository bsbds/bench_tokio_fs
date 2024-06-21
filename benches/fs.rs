use std::future::Future;
use std::hint::black_box;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::io::AsyncWriteExt;
use tokio::runtime::{Builder, Runtime};

const NUM_WRITES: usize = 10;
const WRITE_SIZE: usize = 256 * 100;
const NUM_THREAD: usize = 2;

fn build_runtime() -> Runtime {
    let rt = Builder::new_multi_thread()
        .worker_threads(NUM_THREAD)
        .enable_all()
        .build()
        .unwrap();

    rt
}

fn build_runtime_overload() -> Runtime {
    let rt = build_runtime();
    const INTERVAL_EACH_RUN: Duration = Duration::from_millis(1);
    const NUM_TASK: usize = 1000;

    for _ in 0..NUM_TASK {
        rt.spawn(async move {
            loop {
                black_box(cpu_task());
                tokio::time::sleep(INTERVAL_EACH_RUN).await;
            }
        });
    }

    rt
}

fn cpu_task() -> Vec<i64> {
    let mut rng = StdRng::from_seed([0; 32]);
    let mut v: Vec<_> = std::iter::repeat_with(|| rng.gen::<i64>())
        .take(100)
        .collect();
    v.sort();
    v
}

type Task = Box<dyn FnOnce() + Send + 'static>;

struct Thread {
    task_tx: Option<mpsc::Sender<(Task, mpsc::Sender<()>)>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Thread {
    fn init() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || loop {
            let Some((task, fin_tx)): Option<(Task, mpsc::Sender<()>)> = rx.recv().ok() else {
                break;
            };
            fin_tx.send(task()).unwrap();
        });
        Self {
            task_tx: Some(tx),
            handle: Some(handle),
        }
    }

    fn spawn_to_thread(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        let (tx, rx) = mpsc::channel();
        self.task_tx.as_ref().unwrap().send((task, tx)).unwrap();
        rx.recv().unwrap();
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        drop(self.task_tx.take());
        self.handle.take().unwrap().join().unwrap();
    }
}

async fn spawn_to_runtime(fut: impl Future<Output = ()> + Send + 'static) {
    let handle = tokio::spawn(fut);
    handle.await.unwrap();
}

async fn fs_write_async() {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("fs_write.log")
        .await
        .unwrap();
    for _ in 0..NUM_WRITES {
        file.write_all(&[0; WRITE_SIZE]).await.unwrap();
        file.sync_data().await.unwrap();
    }
    tokio::fs::remove_file("fs_write.log").await.unwrap();
}

async fn fs_write() {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("fs_write.log")
        .unwrap();
    for _ in 0..NUM_WRITES {
        file.write_all(&[0; WRITE_SIZE]).unwrap();
        file.sync_data().unwrap();
    }
    std::fs::remove_file("fs_write.log").unwrap();
}

fn fs_write_thread() {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("fs_write.log")
        .unwrap();
    for _ in 0..NUM_WRITES {
        file.write_all(&[0; WRITE_SIZE]).unwrap();
        file.sync_data().unwrap();
    }
    std::fs::remove_file("fs_write.log").unwrap();
}

fn fs_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_benchmarks");

    let rts = vec![
        (build_runtime(), "noload"),
        (build_runtime_overload(), "stress"),
    ];

    for (rt, name) in &rts {
        let thread = Thread::init();
        group.bench_function(BenchmarkId::new("fs_write_thread", name), |b| {
            b.iter(|| thread.spawn_to_thread(Box::new(fs_write_thread)))
        });
        group.bench_function(BenchmarkId::new("fs_write", name), |b| {
            b.to_async(rt).iter(|| spawn_to_runtime(fs_write()))
        });
        group.bench_function(BenchmarkId::new("fs_write_async", name), |b| {
            b.to_async(rt).iter(|| spawn_to_runtime(fs_write_async()))
        });
    }

    group.finish();
}

criterion_group!(benches, fs_benchmark,);
criterion_main!(benches);
