.PHONY: all

all: fuzzer-libfuzzer

fuzzer-system-libfuzzer: Dockerfile.system
	docker build -t fuzzer-system:libfuzzer -f Dockerfile.system .


fuzzer-libfuzzer-binutils: fuzzer-system-libfuzzer
	docker build -t fuzzer-libfuzzer:binutils -f Dockerfile.binutils .

fuzzer-libfuzzer-lava: fuzzer-system-libfuzzer
	docker build -t fuzzer-libfuzzer:lava -f Dockerfile.lava .

fuzzer-libfuzzer-google: fuzzer-system-libfuzzer
	docker build -t fuzzer-libfuzzer:google -f Dockerfile.google .

#fuzzer-libfuzzer: fuzzer-libfuzzer-binutils fuzzer-libfuzzer-lava fuzzer-libfuzzer-google
fuzzer-libfuzzer: fuzzer-libfuzzer-google
	docker build -t fuzzer-libfuzzer -f Dockerfile.all .
