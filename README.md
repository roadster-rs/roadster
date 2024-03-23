# Roadster

# Start local DB

```shell
# Dev
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
# Test
docker run -d -p 5433:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_test -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
```

# Start local Redis instance

```shell
# Dev
docker run -d -p 6379:6379 redis:7.2-alpine
# Test
docker run -d -p 6380:6379 redis:7.2-alpine
```