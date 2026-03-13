use crate::chunker::chunker::ChunkFactory;
use crate::trace::hashers::HasherFactory;
use crate::tui::tui::{FileStatus, TraceUiState};

use crossbeam_channel::{bounded, Receiver};
use dashmap::{DashMap, DashSet};
use memmap2::{Advice, Mmap};

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
}

fn spawnWorkers(
    numWorkers: usize,
    receiver: Receiver<ChunkingTask>,
    hashSet: Arc<DashSet<[u8; 32]>>,
    fileStats: Arc<DashMap<String, FileStatus>>,
    completedTasks: Arc<AtomicUsize>,
    globalChunkCount: Arc<AtomicUsize>,
    globalDupCount: Arc<AtomicUsize>,
    globalDupSize: Arc<AtomicUsize>,
    hasherFactory: Arc<HasherFactory>,
    chunkFactory: Arc<ChunkFactory>,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(numWorkers);

    for _ in 0..numWorkers {
        let receiver = receiver.clone();
        let hashSet = Arc::clone(&hashSet);
        let fileStats = Arc::clone(&fileStats);
        let completedTasks = Arc::clone(&completedTasks);
        let globalChunkCount = Arc::clone(&globalChunkCount);
        let globalDupCount = Arc::clone(&globalDupCount);
        let globalDupSize = Arc::clone(&globalDupSize);
        let chunker = chunkFactory.createChunker();
        let hasher = hasherFactory.createHasher();

        let handle = thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                // Mark file as in-progress on first task picked up for it
                if let Some(mut s) = fileStats.get_mut(&task.fileName) {
                    if matches!(*s, FileStatus::Queued) {
                        *s = FileStatus::Processing;
                    }
                }

                let fileContent = &task.mmap[task.offset..task.offset + task.length];
                let chunks = chunker.chunk(fileContent);

                let mut localChunkCount: usize = 0;
                let mut localDupCount: usize = 0;
                let mut localDupSize: usize = 0;

                for chunk in chunks {
                    localChunkCount += 1;
                    let hash = hasher.hash(chunk);
                    if !hashSet.insert(hash) {
                        localDupCount += 1;
                        localDupSize += chunk.len();
                    }
                }

                // Flush per task (once per 64 MiB) so TUI stats stay live
                globalChunkCount.fetch_add(localChunkCount, Ordering::Relaxed);
                globalDupCount.fetch_add(localDupCount, Ordering::Relaxed);
                globalDupSize.fetch_add(localDupSize, Ordering::Relaxed);

                // Mark file done when its last work unit completes
                if task.mmap.len() == task.offset + task.length {
                    if let Some(mut s) = fileStats.get_mut(&task.fileName) {
                        *s = FileStatus::Done;
                    }
                }

                completedTasks.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    handles
}

pub fn run(args: &TraceArgs) -> Result<()> {
    let mut tasks: Vec<ChunkingTask> = Vec::new();
    let hasherFactory = Arc::new(HasherFactory::new(args.hashType));
    let chunkFactory = Arc::new(ChunkFactory::new(args.chunkerType));
    let fileStats: Arc<DashMap<String, FileStatus>> = Arc::new(DashMap::new());

    for filename in &args.fileNames {
        let file = File::open(filename)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let _ = mmap.advise(Advice::Sequential);
        let mmap = Arc::new(mmap);
        let fileLength = mmap.len();
        let fname: String = filename.to_string_lossy().into_owned();

        let mut offset = 0;
        while offset < fileLength {
            let length = min(WORK_UNIT_SIZE, fileLength - offset);
            tasks.push(ChunkingTask {
                mmap: Arc::clone(&mmap),
                offset,
                length,
                fileName: fname.clone(),
            });
            offset += length;
        }

        fileStats.insert(fname, FileStatus::Queued);
    }

    let numTasks = tasks.len();
    let numFiles = fileStats.len();
    let numWorkers = args.jobs.unwrap_or(1);

    let (sender, receiver) = bounded(numWorkers * 4);
    let hashSet: Arc<DashSet<[u8; 32]>> = Arc::new(DashSet::new());
    let completedTasks = Arc::new(AtomicUsize::new(0));
    let chunkCount = Arc::new(AtomicUsize::new(0));
    let dupCount = Arc::new(AtomicUsize::new(0));
    let dupSize = Arc::new(AtomicUsize::new(0));
    let isDone = Arc::new(AtomicBool::new(false));

    let uiState = Arc::new(TraceUiState {
        totalTasks: numTasks,
        totalFiles: numFiles,
        completedTasks: Arc::clone(&completedTasks),
        chunkCount: Arc::clone(&chunkCount),
        dupCount: Arc::clone(&dupCount),
        dupSize: Arc::clone(&dupSize),
        fileStats: Arc::clone(&fileStats),
        isDone: Arc::clone(&isDone),
        chunkerLabel: format!("{:?}", args.chunkerType),
        hasherLabel: format!("{:?}", args.hashType),
        numWorkers,
    });

    let terminal = crate::tui::tui::init(numFiles);
    let tuiHandle = {
        let state = Arc::clone(&uiState);
        thread::spawn(move || crate::tui::tui::run(terminal, state))
    };

    let workers = spawnWorkers(
        numWorkers,
        receiver,
        Arc::clone(&hashSet),
        Arc::clone(&fileStats),
        Arc::clone(&completedTasks),
        Arc::clone(&chunkCount),
        Arc::clone(&dupCount),
        Arc::clone(&dupSize),
        Arc::clone(&hasherFactory),
        Arc::clone(&chunkFactory),
    );

    for task in tasks {
        sender.send(task).unwrap();
    }
    drop(sender);

    for (i, worker) in workers.into_iter().enumerate() {
        if let Err(e) = worker.join() {
            eprintln!("Error joining worker thread {}: {:?}", i, e);
        }
    }

    // All workers have finished — mark every file as Done before the final TUI draw.
    // This handles edge cases like empty files that produced no tasks and never
    // transitioned out of Queued.
    for mut entry in fileStats.iter_mut() {
        *entry.value_mut() = FileStatus::Done;
    }

    // Signal TUI after all workers finish so the final draw reflects accurate state
    isDone.store(true, Ordering::Relaxed);
    tuiHandle.join().unwrap();

    println!(
        "Found {} duplicate chunks out of {} total ({} saved).",
        dupCount.load(Ordering::Relaxed),
        chunkCount.load(Ordering::Relaxed),
        fmtSize(dupSize.load(Ordering::Relaxed)),
    );

    Ok(())
}

fn fmtSize(bytes: usize) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KiB", b / KIB)
    } else {
        format!("{} B", bytes)
    }
}
