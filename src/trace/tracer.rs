use crate::chunker::chunker::ChunkFactory;
use crate::trace::hashers::HasherFactory;

use crossbeam::queue::ArrayQueue;
use dashmap::DashSet;
use memmap2::Mmap;

use crate::util::arguments::TraceArgs;

use std::{
    fs::File,
    io::Result,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

struct ChunkingTask {
    mmap: Arc<Mmap>,
    offset: usize,
    length: usize,
    hasherFactory: Arc<HasherFactory>,
    chunkFactory: Arc<ChunkFactory>,
}

fn spawnWorkers(
    numWorkers: usize,
    queue: Arc<ArrayQueue<ChunkingTask>>,
    hashSet: Arc<DashSet<Vec<u8>>>,
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
                        let chunks = task.chunkFactory.createChunker().chunk(task.mmap.as_ref());
                        let hasher = task.hasherFactory.createHasher();

                        for chunk in chunks {
                            let hash = hasher.hash(chunk);
                            if !hashSet.insert(hash) {
                                localDupCount += 1;
                            }
                        }
                    }
                    None => {
                        if isDone.load(Ordering::Relaxed) {
                            globalDupCount.fetch_add(localDupCount, Ordering::Relaxed);
                            break;
                        }
                        // TODO: Tune the sleep time for best performance
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
    // TODO: This now treats every (file, chunker) combination as a single task, but we want to
    // split large files into multiple tasks to get balanced threads.
    let numTasks: usize = args.fileNames.len() * args.chunkerTypes.len();
    let hasherFactory: Arc<HasherFactory> = Arc::new(HasherFactory::new(args.hashType));
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

            let chunkFactory: Arc<ChunkFactory> =
                Arc::new(ChunkFactory::new(chunker.clone(), args.hashSalt.clone()));

            let offset: usize = 0;
            let length: usize = 0;

            let task = ChunkingTask {
                mmap: Arc::new(mmap),
                offset,
                length,
                hasherFactory: Arc::clone(&hasherFactory),
                chunkFactory: Arc::clone(&chunkFactory),
            };

            if let Err(_) = queue.push(task) {
                eprintln!("Failed to add task to queue - queue is full!");
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
