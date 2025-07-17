# Worker benchmarks

Benchmarks for processing async jobs with Roadster's workers.

# Running locally

```shell
# Start the database and redis (for sidekiq). Note: change the credentials when deploying to prod
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=example_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
docker run -d -p 6379:6379 redis:7.2-alpine
# Run the benchmarks
ROADSTER__ENVIRONMENT=test cargo bench
```
