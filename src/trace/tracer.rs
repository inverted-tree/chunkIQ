use crossbeam::queue::ArrayQueue;

use crate::{ChunkerType, HashType, TraceArgs};

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Clone)]
struct ChunkerConfig {
    chunkerType: ChunkerType,
    hashType: HashType,
    hashSalt: Option<String>,
}

struct ChunkingTask {
    filename: PathBuf,
    config: ChunkerConfig,
}

fn spawnWorkers(
    numWorkers: usize,
    queue: Arc<ArrayQueue<ChunkingTask>>,
    isDone: Arc<AtomicBool>,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(numWorkers);

    for workerIdx in 0..numWorkers {
        let queue = Arc::clone(&queue);
        let isDone = Arc::clone(&isDone);

        let handle = thread::spawn(move || {
            println!("Worker {}: Started.", workerIdx);
            loop {
                match queue.pop() {
                    Some(task) => {
                        println!(
                            "Worker {}: Chunking {:?} with {:?} using {:?} salted with {:?}.",
                            workerIdx,
                            task.filename,
                            task.config.chunkerType,
                            task.config.hashType,
                            task.config.hashSalt.unwrap_or("no salt".to_string())
                        );
                    }
                    None => {
                        if isDone.load(Ordering::Relaxed) {
                            break;
                        }

                        println!("Worker {}: No task in queue. Sleeping.", workerIdx);
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
        handles.push(handle);
    }
    handles
}

pub fn run(args: &TraceArgs) {
    let numTasks: usize = args.fileNames.len() * args.chunkerTypes.len();
    let queue: Arc<ArrayQueue<ChunkingTask>> = Arc::new(ArrayQueue::new(numTasks));
    let isDone = Arc::new(AtomicBool::new(false));

    let workers = spawnWorkers(
        args.jobs.unwrap_or(1),
        Arc::clone(&queue),
        Arc::clone(&isDone),
    );

    for file in &args.fileNames {
        for chunker in &args.chunkerTypes {
            let task = ChunkingTask {
                filename: file.clone(),
                config: ChunkerConfig {
                    chunkerType: chunker.clone(),
                    hashType: args.digestType,
                    hashSalt: args.hashSalt.clone(),
                },
            };
            match queue.push(task) {
                Ok(_) => (),
                Err(_) => eprintln!("Failed to add task to queue - queue is full!"),
            }
        }
    }
    isDone.store(true, Ordering::Relaxed);

    for (i, worker) in workers.into_iter().enumerate() {
        if let Err(e) = worker.join() {
            eprintln!("Error joining worker thread {}: {:?}", i, e);
        } else {
            println!("Main thread: Worker {} completed successfully", i);
        }
    }
}
