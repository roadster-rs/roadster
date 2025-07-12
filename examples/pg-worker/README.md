# App Builder Example

An example of using the Postgres worker backend instead of the Sidekiq backend.

# Running locally

```shell
# Set the environment, example:
export ROADSTER__ENVIRONMENT=development
# Start the database. Note: change the credentials when deploying to prod
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=example_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
# Start the app
cargo run
```
