//! Background task queue service backed by Postgres using [pgmq](https://docs.rs/pgmq).

/*
- job/task/message
- runner/worker/handler/processor
 */
pub mod processor;

use crate::error::RoadsterResult;
use async_trait::async_trait;

#[async_trait]
pub trait Worker<Args> {
    // todo: Make this roadster specific and pass the app-state as a method param? That would
    //  certainly make it a bit easier to use, which would be nice.
    // todo: Make general enough to work as a shared/wrapper trait of sidekiq's worker trait?
    async fn handle(args: Args) -> RoadsterResult<()>;
}

/*
Lifecycle
    Init
        Create DB conn pool based on config
        Create PGMQueue instance
        Register workers
        Create queue tables
        Start worker threads/executors based on config

    Enqueue jobs
        Send* methods

    Handle jobs
        For each queue
            Read a message from the queue, with vt set to job timeout + backoff strategy delay
            If message returned, timeout (configurable) and query the next queue
            Get the worker instance for the job and call its "handle" method
            If job succeeds, delete or archive the message based on config
            If job fails/panics and retry count has exceeded, delete or archive the message based on config
            Yield and query the next queue

    App shutdown
        In queue fetching, listen for app shutdown signal, and stop loop on shutdown




Comparison of PGMQueue vs PGMQueueExt methods.

I think we'll want to start with just supporting PGMQueue for now without the extension. PGMQueue
can do everything PGMQueueExt can do except create partitioned tables, and PGMQueueExt doesn't
support sending batches of messages.

PGMQueue
    archive
    archive_batch
    create
    create_unlogged
    delete
    delete_batch
*   destroy -- I think this is the same as `PGMQueueExt#drop_queue`
    new
    new_with_pool
    pop
    purge -- Deletes all messages in the queue(?). I think this is the same as `PGMQueueExt#purge_queue`
    read
*   read_batch
    read_batch_with_poll
    send
*   send_batch
*   send_batch_delay
    send_delay
    set_vt

PGMQueueExt
    archive
    archive_batch
    create
*   create_partitioned
    create_unlogged
    delete
    delete_batch
*   drop_queue -- I think this is the same as `PGMQueue#destroy`
*   init -- Inits the pgmq pg extension
*   list_queues
    new
    new_with_pool
    pop
    purge_queue -- rustdoc is the same as `drop_queue`, but I'm guessing this is the same as `PGMQueueExt#purge_queue`
    read
    read_batch_with_poll
    send
    send_delay
    set_vt

 */
