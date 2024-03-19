# Integrationtests

This crate contains integration tests for moksha.

## Prerequisites

Before running the tests, ensure that you have the following software installed on your machine:

- Docker: Our tests use Docker to create isolated, reproducible environments. You can download Docker from the [official website](https://www.docker.com/products/docker-desktop).
- Docker Compose: This is a tool for defining and running multi-container Docker applications. It's included in the Docker Desktop installation for Windows and Mac. For Linux, you can follow the instructions on the [official documentation](https://docs.docker.com/compose/install/).

## Running the Tests

Before running the tests you have to start docker containers for the services that the tests depend on. You can do this by running the following command:

```bash
docker compose --profile itest up -d
```

To run the integration tests, use the `itests` command in your terminal:

```bash
just run-itests
```

Please note that the first time you run the tests, Docker may need to download the required images. This can take some time, but the images will be cached for future runs.
