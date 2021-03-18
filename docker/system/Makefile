.PHONY: all

DOCKER_FLAGS ?=

all: fuzzer-system

fuzzer-system: Dockerfile
	docker build $(DOCKER_FLAGS) -t fuzzer-system .
