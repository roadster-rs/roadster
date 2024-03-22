# Roadster

# Start local DB

```shell
# Dev
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
# Test
docker run -d -p 5433:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_test -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
```
