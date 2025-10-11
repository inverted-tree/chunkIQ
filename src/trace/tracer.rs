use crate::chunker::chunker::ChunkFactory;
use crate::trace::hashers::HasherFactory;

// use crate::tui::tui;
use crossbeam_channel::{bounded, Receiver};
use dashmap::{DashMap, DashSet};
use memmap2::Mmap;

use crate::util::arguments::TraceArgs;

use std::{
    cmp::min,
    fs::File,
    io::Result,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

const WORK_UNIT_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub struct ChunkingTask {
    mmap: Arc<Mmap>,
    offset: usize,
    length: usize,
    fileName: String,
    hasherFactory: Arc<HasherFactory>,
    chunkFactory: Arc<ChunkFactory>,
}

fn spawnWorkers(
    numWorkers: usize,
    receiver: Receiver<ChunkingTask>,
    hashSet: Arc<DashSet<Vec<u8>>>,
    fileStats: Arc<DashMap<String, bool>>,
    globalChunkCount: Arc<AtomicUsize>,
    globalDupCount: Arc<AtomicUsize>,
    globalDupSize: Arc<AtomicUsize>,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(numWorkers);

    for _workerIdx in 0..numWorkers {
        let receiver = receiver.clone();
        let hashSet = Arc::clone(&hashSet);
        let fileStats = Arc::clone(&fileStats);
        let globalChunkCount = Arc::clone(&globalChunkCount);
        let globalDupCount = Arc::clone(&globalDupCount);
        let globalDupSize = Arc::clone(&globalDupSize);

        let handle = thread::spawn(move || {
            let mut localChunkCount: usize = 0;
            let mut localDupCount: usize = 0;
            let mut localDupSize: usize = 0;

            while let Ok(task) = receiver.recv() {
                let fileContent = &task.mmap[task.offset..task.offset + task.length];
                let chunks = task.chunkFactory.createChunker().chunk(fileContent);
                let hasher = task.hasherFactory.createHasher();

                for chunk in chunks {
                    localChunkCount += 1;
                    let hash = hasher.hash(chunk);
                    if !hashSet.insert(hash) {
                        localDupCount += 1;
                        localDupSize += chunk.len();
                    }
                }

                if task.mmap.len() == task.offset + task.length {
                    fileStats
                        .entry(task.fileName.clone())
                        .and_modify(|v| *v = true)
                        .or_insert(true);
                }
            }

            globalChunkCount.fetch_add(localChunkCount, Ordering::Relaxed);
            globalDupCount.fetch_add(localDupCount, Ordering::Relaxed);
            globalDupSize.fetch_add(localDupSize, Ordering::Relaxed);
        });
        handles.push(handle);
    }

    handles
}

pub fn run(args: &TraceArgs) -> Result<()> {
    let mut tasks: Vec<ChunkingTask> = Vec::new();
    let hasherFactory: Arc<HasherFactory> = Arc::new(HasherFactory::new(args.hashType));
    let fileStats: Arc<DashMap<String, bool>> = Arc::new(DashMap::new());

    for filename in &args.fileNames {
        let file = File::open(filename)?;
        let mmap = Arc::new(unsafe { Mmap::map(&file)? });
        let fileLength = mmap.len();
        let fname: String = filename.to_string_lossy().into_owned();

        let chunkFactory: Arc<ChunkFactory> = Arc::new(ChunkFactory::new(args.chunkerType.clone()));

        let mut offset = 0;
        while offset < fileLength {
            let length = min(WORK_UNIT_SIZE, fileLength - offset);
            tasks.push(ChunkingTask {
                mmap: Arc::clone(&mmap),
                offset,
                length,
                fileName: fname.clone(),
                hasherFactory: Arc::clone(&hasherFactory),
                chunkFactory: Arc::clone(&chunkFactory),
            });
            offset += length;
        }

        fileStats.insert(fname.clone(), false);
    }

    let numTasks = tasks.len();
    let (sender, receiver) = bounded(numTasks);
    let hashSet = Arc::new(DashSet::new());
    let chunkCount = Arc::new(AtomicUsize::new(0));
    let dupCount = Arc::new(AtomicUsize::new(0));
    let dupSize = Arc::new(AtomicUsize::new(0));
    let isDone = Arc::new(AtomicBool::new(false));

    let workers = spawnWorkers(
        args.jobs.unwrap_or(1),
        receiver.clone(),
        Arc::clone(&hashSet),
        Arc::clone(&fileStats),
        Arc::clone(&chunkCount),
        Arc::clone(&dupCount),
        Arc::clone(&dupSize),
    );

    // let tuiHandle = {
    //     let receiver = receiver.clone();
    //     let isDone = Arc::clone(&isDone);
    //     let fileStats = Arc::clone(&fileStats);
    //     thread::spawn(move || {
    //         tui::initAndRunTrace(numTasks, receiver, isDone, fileStats);
    //     })
    // };

    for task in tasks {
        sender.send(task).unwrap();
    }
    drop(sender);
    isDone.store(true, Ordering::Relaxed);

    for (i, worker) in workers.into_iter().enumerate() {
        if let Err(e) = worker.join() {
            eprintln!("Error joining worker thread {}: {:?}", i, e);
        }
    }
    // tuiHandle.join().unwrap();

    println!(
        "Found a total of {} duplicate chunks out of a total of {} chunks which accounts to {}KiB.",
        dupCount.load(Ordering::Relaxed),
        chunkCount.load(Ordering::Relaxed),
        dupSize.load(Ordering::Relaxed) >> 10
    );

    Ok(())
}
