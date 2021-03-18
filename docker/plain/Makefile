.PHONY: all

DOCKER_FLAGS ?=

all: fuzzer-plain

fuzzer-system-plain: Dockerfile.system
	docker build ${DOCKER_FLAGS} -t fuzzer-system:plain -f Dockerfile.system .


fuzzer-plain-binutils: fuzzer-system-plain
	docker build ${DOCKER_FLAGS} -t fuzzer-plain:binutils -f Dockerfile.binutils .

fuzzer-plain-lava: fuzzer-system-plain
	docker build ${DOCKER_FLAGS} -t fuzzer-plain:lava -f Dockerfile.lava .

fuzzer-plain-google: fuzzer-system-plain
	docker build ${DOCKER_FLAGS} -t fuzzer-plain:google -f Dockerfile.google .

fuzzer-plain: fuzzer-plain-binutils # fuzzer-plain-lava fuzzer-plain-google
	docker build ${DOCKER_FLAGS} -t fuzzer-plain -f Dockerfile.all .
