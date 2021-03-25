.PHONY: all

all: fuzzer-afl

fuzzer-system-afl: Dockerfile.system
	docker build -t fuzzer-system:afl -f Dockerfile.system .


fuzzer-afl-binutils: fuzzer-system-afl
	docker build -t fuzzer-afl:binutils -f Dockerfile.binutils .

fuzzer-afl-lava: fuzzer-system-afl
	docker build -t fuzzer-afl:lava -f Dockerfile.lava .

fuzzer-afl-google: fuzzer-system-afl
	docker build -t fuzzer-afl:google -f Dockerfile.google .

fuzzer-afl: fuzzer-afl-binutils fuzzer-afl-google #fuzzer-afl-lava 
	docker build -t fuzzer-afl -f Dockerfile.all .
