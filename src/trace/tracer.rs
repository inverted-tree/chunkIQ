use crate::chunker;

use crossbeam::queue::ArrayQueue;
use dashmap::DashSet;
use digest::{consts::True, Digest, Output};
use memmap2::Mmap;
use sha1::Sha1;

use crate::{ChunkerType, HashType, TraceArgs};

use std::{
    fs::File,
    io::Result,
    string::String,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
    // hashSet: Arc<DashSet<Output<sha1::Sha1>>>,
}

fn spawnWorkers(
    numWorkers: usize,
    queue: Arc<ArrayQueue<ChunkingTask>>,
    hashSet: Arc<DashSet<Output<sha1::Sha1>>>,
    globalDupCount: Arc<AtomicUsize>,
    isDone: Arc<AtomicBool>,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(numWorkers);

    for _workerIdx in 0..numWorkers {
        let queue = Arc::clone(&queue);
        let hashSet = Arc::clone(&hashSet);
        let globalDupCount = Arc::clone(&globalDupCount);
        let isDone = Arc::clone(&isDone);

        let handle = thread::spawn(move || {
            let mut localDupCount: usize = 0;
            loop {
                match queue.pop() {
                    Some(task) => {
                        let mut hasher = Sha1::new();
                        let chunks = task.mmap.chunks(task.config.chunkerType.getSize());

                        for chunk in chunks {
                            hasher.update(chunk);

                            let hash = hasher.finalize_reset();

                            if hashSet.contains(&hash) {
                                localDupCount += 1;
                            } else {
                                hashSet.insert(hash);
                            }
                        }
                    }
                    None => {
                        if isDone.load(Ordering::Relaxed) {
                            globalDupCount.fetch_add(localDupCount, Ordering::Relaxed);
                            break;
                        }
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
    let hashSet = Arc::new(DashSet::new());
    let dupCount = Arc::new(AtomicUsize::new(0));
    let isDone = Arc::new(AtomicBool::new(false));

    let workers = spawnWorkers(
        args.jobs.unwrap_or(1),
        Arc::clone(&queue),
        Arc::clone(&hashSet),
        Arc::clone(&dupCount),
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
        }
    }
    println!(
        "Found a total of {} duplicate chunks which accounts to {}KiB.",
        dupCount.load(Ordering::Relaxed),
        dupCount.load(Ordering::Relaxed) * args.chunkerTypes[0].getSize() >> 10
    );

    Ok(())
}
