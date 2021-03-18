.PHONY: all

all: fuzzer-honggfuzz

fuzzer-system-honggfuzz: Dockerfile.system
	docker build -t fuzzer-system:honggfuzz -f Dockerfile.system .


fuzzer-honggfuzz-binutils: fuzzer-system-honggfuzz
	docker build -t fuzzer-honggfuzz:binutils -f Dockerfile.binutils .

fuzzer-honggfuzz-lava: fuzzer-system-honggfuzz
	docker build -t fuzzer-honggfuzz:lava -f Dockerfile.lava .

fuzzer-honggfuzz-google: fuzzer-system-honggfuzz
	docker build -t fuzzer-honggfuzz:google -f Dockerfile.google .

fuzzer-honggfuzz: fuzzer-honggfuzz-binutils fuzzer-honggfuzz-lava fuzzer-honggfuzz-google
	docker build -t fuzzer-honggfuzz -f Dockerfile.all .
