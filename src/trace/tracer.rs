use crossbeam::channel::{Receiver, Sender, unbounded};

use crate::{ChunkerType, HashType, TraceArgs};

use std::{path::PathBuf, thread};

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

// struct WorkerContext {
//     chunkers: HashMap<ChunkerConfig, Box<dyn Chunker>>,
//     bufferPool: BufferPool,
// }
//
// trait Chunker {
//     fn reset(&mut self);
//     fn chunkFile(&mut self, file &PathBuf, output: &mut dyn ChunkSink) -> Result<()>;
// }

fn initWorkerPool<F>(workerCount: usize, taskHandler: F) -> Sender<ChunkingTask>
where
    F: Fn(ChunkingTask) + Send + Sync + 'static + Clone,
{
    let (sender, reciever): (Sender<ChunkingTask>, Receiver<ChunkingTask>) = unbounded();

    for _ in 0..workerCount {
        let reciever = reciever.clone();
        let handler = taskHandler.clone();

        thread::spawn(move || {
            while let Ok(task) = reciever.recv() {
                handler(task);
            }
        });
    }

    sender
}

pub fn run(args: &TraceArgs) {
    // This is just a dummy chunking chunking job
    let chunkFile = |task: ChunkingTask| {
        println!(
            "Chunking file {:?} with config [ ct: {:?} ; ht: {:?} ; hs {:?} ]",
            task.filename, task.config.chunkerType, task.config.hashType, task.config.hashSalt
        );
    };

    let sender = initWorkerPool(args.jobs.unwrap(), chunkFile);

    let configs = vec![
        ChunkerConfig {
            chunkerType: args.chunkerType,
            hashType: args.digestType,
            hashSalt: args.hashSalt.clone(),
        },
        ChunkerConfig {
            chunkerType: args.chunkerType,
            hashType: args.digestType,
            hashSalt: args.hashSalt.clone(),
        },
    ];

    for file in &args.fileNames {
        for conf in &configs {
            let task = ChunkingTask {
                filename: file.clone(),
                config: conf.clone(),
            };
            sender.send(task).expect("Failed to send task");
        }
    }
    thread::sleep(std::time::Duration::from_millis(10));
}
