.PHONY: docker framework tools

REMOTE_REGISTRY ?= sarek.osterlund.xyz

all: submodules framework docker tools

docker:
	$(MAKE) -C docker

framework: framework-binutils framework-lava framework-google

framework-binutils:
	DOCKER_BUILDKIT=1 docker build --target=framework-binutils --tag=fuzzer-framework-binutils .

framework-lava:
	DOCKER_BUILDKIT=1 docker build --target=framework-lava --tag=fuzzer-framework-lava .

framework-google:
	DOCKER_BUILDKIT=1 docker build --target=framework-google --tag=fuzzer-framework-google .

submodules:
	git submodule update --init --recursive

tools: afl_generic_driver collab_fuzz_runner

afl_generic_driver:
	$(MAKE) -C drivers/afl_generic docker

collab_fuzz_runner:
	$(MAKE) -C runners

remote_push: framework afl_generic_driver
	docker tag fuzzer-framework ${REMOTE_REGISTRY}/fuzzer-framework
	docker tag fuzzer-generic-driver ${REMOTE_REGISTRY}/fuzzer-generic-driver
	docker push ${REMOTE_REGISTRY}/fuzzer-framework
	docker push ${REMOTE_REGISTRY}/fuzzer-generic-driver
	$(MAKE) -C docker remote_push

remote_pull:
	docker pull ${REMOTE_REGISTRY}/fuzzer-framework
	docker pull ${REMOTE_REGISTRY}/fuzzer-generic-driver
	docker tag ${REMOTE_REGISTRY}/fuzzer-framework fuzzer-framework
	docker tag ${REMOTE_REGISTRY}/fuzzer-generic-driver fuzzer-generic-driver
	$(MAKE) -C docker remote_pull

