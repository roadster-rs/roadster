use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fake::Fake;
use fake::rand::random;
use itertools::Itertools;
use roadster::app::PrepareOptions;
use roadster::app::context::AppContext;
use roadster::service::Service;
use roadster::service::worker::PgWorkerService;
use roadster::worker::Worker;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use worker_bench::build_app;
use worker_bench::latch::Countdown;
use worker_bench::worker::example::{ExampleWorkerArgs, PgExampleWorker, SidekiqExampleWorker};

async fn run_jobs<W: 'static + Worker<AppContext, ExampleWorkerArgs>>(
    state: &AppContext,
    num_jobs: u32,
    latch_wait: impl Future + Send,
) -> Duration {
    let jobs = std::iter::repeat_with(|| {
        ExampleWorkerArgs::builder()
            .foo(fake::faker::name::raw::Name(fake::locales::EN).fake::<String>())
            .bar(random())
            .build()
    })
    .take(num_jobs as usize)
    .collect_vec();

    let timer = Instant::now();
    W::enqueue_batch(state, &jobs).await.unwrap();

    latch_wait.await;

    timer.elapsed()
}

fn worker_benchmark<W: 'static + Worker<AppContext, ExampleWorkerArgs>>(
    c: &mut Criterion,
    name: &str,
    num_workers: impl IntoIterator<Item = u32>,
) {
    const NUM_JOBS: u32 = 1_000;

    let runtime = Runtime::new().expect("Failed to create runtime");

    let mut group = c.benchmark_group(format!("worker-{name}"));

    group.sample_size(10);
    group.measurement_time(Duration::from_secs(35));

    for num_workers in num_workers {
        group.throughput(Throughput::Elements(NUM_JOBS as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_workers),
            &num_workers,
            |b, num_workers| {
                b.iter_custom(|num_iterations| {
                    let mut duration = Duration::from_secs(0);
                    for _i in 0..num_iterations {
                        let (latch, latch_wait) = Countdown::new(NUM_JOBS);
                        let cancellation_token = CancellationToken::new();
                        let app = build_app(latch, cancellation_token.clone());
                        let app = runtime.block_on(async move {
                            roadster::app::prepare(
                                app,
                                PrepareOptions::builder()
                                    .add_config_source(
                                        roadster::config::ConfigOverrideSource::builder()
                                            .name(&format!("service.worker.{name}.num-workers"))
                                            .value(*num_workers)
                                            .build(),
                                    )
                                    .add_config_source(
                                        roadster::config::ConfigOverrideSource::builder()
                                            .name(&format!("service.worker.{name}.enable"))
                                            .value(true)
                                            .build(),
                                    )
                                    .build(),
                            )
                            .await
                            .unwrap()
                        });

                        let state = app.state.clone();

                        runtime.block_on(async {
                            let srv = app
                                .service_registry
                                .get::<PgWorkerService<AppContext>>()
                                .unwrap();
                            srv.before_run(&state).await.unwrap();
                            sqlx::query(&format!(
                                "truncate pgmq.q_{}",
                                worker_bench::worker::example::QUEUE
                            ))
                            .execute(&state.pgmq().connection)
                            .await
                            .unwrap();
                        });

                        let app_handle = runtime.spawn(async move {
                            roadster::app::run_prepared(app).await.unwrap();
                        });
                        duration += runtime.block_on(run_jobs::<W>(&state, NUM_JOBS, latch_wait));
                        cancellation_token.cancel();
                        runtime.block_on(app_handle).unwrap();
                    }
                    duration
                })
            },
        );
    }

    group.finish();
}

fn pg_worker_benchmark(c: &mut Criterion) {
    worker_benchmark::<PgExampleWorker>(
        c,
        "pg",
        [1u32, 2u32, 5u32, 10u32, 100u32, 200u32, 500u32, 1_000u32],
    )
}

fn sidekiq_worker_benchmark(c: &mut Criterion) {
    worker_benchmark::<SidekiqExampleWorker>(
        c,
        "sidekiq",
        /*
        If the number of workers exceeds the number of Redis connections available in the pool,
        workers will start to time out waiting to acquire a connection from the pool, so we only
        benchmark with a small number of workers.
         */
        [1u32, 2u32, 5u32, 10u32],
    )
}

criterion_group!(benches, pg_worker_benchmark, sidekiq_worker_benchmark);
criterion_main!(benches);
