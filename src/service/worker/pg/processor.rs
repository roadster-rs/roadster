use pgmq::PGMQueue;

#[non_exhaustive]
struct Processor {
    queue: PGMQueue,
}
