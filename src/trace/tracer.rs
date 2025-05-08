use crate::chunker;

use crossbeam::queue::ArrayQueue;
use memmap2::Mmap;

use crate::{ChunkerType, HashType, TraceArgs};

use std::{
    fs::File,
    io::Result,
    string::String,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
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
    mmap: Arc<Mmap>,
    offset: usize,
    length: usize,
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
                        let chunks = task.mmap.chunks(task.config.chunkerType.getSize());
                        for chunk in chunks {
                            println!(
                                "Worker {}: Reading chunk: {:?}",
                                workerIdx,
                                std::str::from_utf8(chunk)
                            );
                        }
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

pub fn run(args: &TraceArgs) -> Result<()> {
    let numTasks: usize = args.fileNames.len() * args.chunkerTypes.len();
    let queue: Arc<ArrayQueue<ChunkingTask>> = Arc::new(ArrayQueue::new(numTasks));
    let isDone = Arc::new(AtomicBool::new(false));

    let workers = spawnWorkers(
        args.jobs.unwrap_or(1),
        Arc::clone(&queue),
        Arc::clone(&isDone),
    );

    for fileName in &args.fileNames {
        for chunker in &args.chunkerTypes {
            let file = File::open(fileName)?;
            let mmap = unsafe { Mmap::map(&file)? };

            let offset: usize = 0;
            let length: usize = 0;

            let task = ChunkingTask {
                mmap: Arc::new(mmap),
                offset,
                length,
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

    Ok(())
}
