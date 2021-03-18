.PHONY: all

all: fuzzer-aflplusplus

fuzzer-system-aflplusplus: Dockerfile.system
	docker build -t fuzzer-system:aflplusplus -f Dockerfile.system .


fuzzer-aflplusplus-binutils: fuzzer-system-aflplusplus
	docker build -t fuzzer-aflplusplus:binutils -f Dockerfile.binutils .

fuzzer-aflplusplus-lava: fuzzer-system-aflplusplus
	docker build -t fuzzer-aflplusplus:lava -f Dockerfile.lava .

fuzzer-aflplusplus-google: fuzzer-system-aflplusplus
	docker build -t fuzzer-aflplusplus:google -f Dockerfile.google .

fuzzer-aflplusplus: fuzzer-aflplusplus-binutils # fuzzer-aflplusplus-lava fuzzer-aflplusplus-google
	docker build -t fuzzer-aflplusplus -f Dockerfile.all .
