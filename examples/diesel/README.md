# Full Example

An example of using all (most) of Roadster's features. Used as a reference implementation and testing playground.

# Running locally

```shell
# Set the environment, example:
export ROADSTER__ENVIRONMENT=development
# Start the database and redis (for sidekiq). Note: change the credentials when deploying to prod
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=example_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
docker run -d -p 6379:6379 redis:7.2-alpine
# Start a local smtp server, such as https://github.com/maildev/maildev
docker run -d -p 1080:1080 -p 1025:1025 maildev/maildev
# Start the app
cargo run
```
