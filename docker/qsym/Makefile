.PHONY: all

DOCKER_FLAGS ?=

all: fuzzer-qsym

fuzzer-system-qsym: Dockerfile.system
	docker build $(DOCKER_FLAGS) -t fuzzer-system:qsym -f Dockerfile.system .

fuzzer-google-qsym: Dockerfile.google
	docker build $(DOCKER_FLAGS) -t fuzzer-qsym:google -f Dockerfile.google .

fuzzer-qsym: fuzzer-system-qsym fuzzer-google-qsym
	docker build $(DOCKER_FLAGS) -t fuzzer-qsym -f Dockerfile.all .

